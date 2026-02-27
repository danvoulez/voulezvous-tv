#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
API_URL="${VVTV_API_URL:-http://127.0.0.1:7070}"
CONTROL_TOKEN="${VVTV_CONTROL_TOKEN:-dev-token}"
CONTROL_SECRET="${VVTV_CONTROL_SECRET:-dev-secret}"
BACKUP_KEY_DIR="${VVTV_BACKUP_KEY_DIR:-$ROOT_DIR/runtime/keys}"
BACKUP_KEY_FILE="${VVTV_BACKUP_KEY_FILE:-$BACKUP_KEY_DIR/current.key}"
BACKUP_KEY_ID_FILE="${VVTV_BACKUP_KEY_ID_FILE:-$BACKUP_KEY_DIR/current.key.id}"

usage() {
  cat <<USAGE
Usage: scripts/vvtv-runbook.sh <command>

Commands:
  force-nightly           Run orchestrator once with forced nightly maintenance
  export-audits           Force nightly and print latest audit export file
  backup-key-rotate       Rotate local backup encryption key
  backup-metadata         Snapshot state.db + OwnerCard with manifest checksum
  backup-metadata-secure  Snapshot + encrypt backup payload at rest
  verify-backup           Verify backup manifest/checksums without restoring
  verify-backup-secure    Decrypt + verify encrypted backup
  restore-metadata        Restore state.db + OwnerCard from backup dir
  restore-metadata-secure Decrypt + restore encrypted backup
  emergency-toggle        Toggle emergency mode via signed control API
  emergency-on            Ensure emergency mode is ON
  emergency-off           Ensure emergency mode is OFF
USAGE
}

require_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "missing required command: $1" >&2
    exit 1
  fi
}

sign_request() {
  local method="$1"
  local path="$2"
  local ts="$3"
  local body="$4"
  printf "%s\n%s\n%s\n%s" "$method" "$path" "$ts" "$body" | \
    openssl dgst -sha256 -hmac "$CONTROL_SECRET" -hex | awk '{print $2}'
}

call_signed_control() {
  local path="$1"
  local body="${2:-}"
  local ts sig
  ts="$(date +%s)"
  sig="$(sign_request "POST" "$path" "$ts" "$body")"

  curl -sS -X POST "${API_URL}${path}" \
    -H "Authorization: Bearer ${CONTROL_TOKEN}" \
    -H "x-vvtv-ts: ${ts}" \
    -H "x-vvtv-signature: ${sig}" \
    --data "$body"
}

force_nightly() {
  (cd "$ROOT_DIR" && VVTV_RUN_ONCE=1 VVTV_FORCE_NIGHTLY=1 cargo run -q -p vvtv-orchestrator)
}

backup_metadata() {
  (cd "$ROOT_DIR" && cargo run -q -p vvtv-admin -- backup \
    --state-db "${VVTV_STATE_DB_PATH:-runtime/state/vvtv.db}" \
    --owner-card "${VVTV_OWNER_CARD_PATH:-config/owner_card.sample.yaml}" \
    --output-dir "${VVTV_BACKUP_DIR:-runtime/backups}")
}

backup_key_rotate() {
  mkdir -p "$BACKUP_KEY_DIR/archive"

  if [[ -f "$BACKUP_KEY_FILE" && -f "$BACKUP_KEY_ID_FILE" ]]; then
    local old_id
    old_id="$(cat "$BACKUP_KEY_ID_FILE")"
    cp "$BACKUP_KEY_FILE" "$BACKUP_KEY_DIR/archive/${old_id}.key"
    chmod 600 "$BACKUP_KEY_DIR/archive/${old_id}.key"
  fi

  local new_id
  new_id="$(date -u +%Y%m%dT%H%M%SZ)"
  openssl rand -base64 48 >"$BACKUP_KEY_FILE"
  chmod 600 "$BACKUP_KEY_FILE"
  echo "$new_id" >"$BACKUP_KEY_ID_FILE"
  chmod 600 "$BACKUP_KEY_ID_FILE"

  echo "backup_key_id=$new_id"
  echo "backup_key_file=$BACKUP_KEY_FILE"
}

ensure_backup_key() {
  if [[ ! -f "$BACKUP_KEY_FILE" ]]; then
    echo "missing backup key file: $BACKUP_KEY_FILE" >&2
    echo "run: scripts/vvtv-runbook.sh backup-key-rotate" >&2
    exit 1
  fi
}

backup_metadata_secure() {
  ensure_backup_key

  local output backup_dir key_id tar_plain enc_file
  output="$(backup_metadata)"
  echo "$output"

  backup_dir="$(echo "$output" | sed -n 's/^backup_dir=//p' | tail -n 1)"
  if [[ -z "$backup_dir" ]]; then
    echo "failed to resolve backup_dir from backup output" >&2
    exit 1
  fi

  key_id="unknown"
  if [[ -f "$BACKUP_KEY_ID_FILE" ]]; then
    key_id="$(cat "$BACKUP_KEY_ID_FILE")"
  fi

  tar_plain="$backup_dir/backup.tar.gz"
  enc_file="$backup_dir/backup.tar.gz.enc"

  tar -C "$backup_dir" -czf "$tar_plain" state.db owner_card.yaml manifest.json
  openssl enc -aes-256-cbc -pbkdf2 -salt -in "$tar_plain" -out "$enc_file" -pass "file:$BACKUP_KEY_FILE"
  shasum -a 256 "$enc_file" | awk '{print $1}' >"$backup_dir/backup.tar.gz.enc.sha256"
  echo "$key_id" >"$backup_dir/key_id.txt"

  rm -f "$tar_plain" "$backup_dir/state.db" "$backup_dir/owner_card.yaml" "$backup_dir/manifest.json"

  echo "secure_backup_dir=$backup_dir"
  echo "secure_backup_payload=$enc_file"
}

restore_metadata() {
  local backup_dir="${1:-}"
  if [[ -z "$backup_dir" ]]; then
    echo "missing backup dir: usage scripts/vvtv-runbook.sh restore-metadata <backup_dir>" >&2
    exit 1
  fi

  (cd "$ROOT_DIR" && cargo run -q -p vvtv-admin -- restore \
    --backup-dir "$backup_dir" \
    --state-db "${VVTV_STATE_DB_PATH:-runtime/state/vvtv.db}" \
    --owner-card "${VVTV_OWNER_CARD_PATH:-config/owner_card.sample.yaml}" \
    --force)
}

verify_backup() {
  local backup_dir="${1:-}"
  if [[ -z "$backup_dir" ]]; then
    echo "missing backup dir: usage scripts/vvtv-runbook.sh verify-backup <backup_dir>" >&2
    exit 1
  fi
  (cd "$ROOT_DIR" && cargo run -q -p vvtv-admin -- verify --backup-dir "$backup_dir")
}

resolve_backup_key_for_dir() {
  local backup_dir="$1"

  if [[ -f "$BACKUP_KEY_FILE" ]]; then
    echo "$BACKUP_KEY_FILE"
    return 0
  fi

  if [[ -f "$backup_dir/key_id.txt" ]]; then
    local key_id archive_key
    key_id="$(cat "$backup_dir/key_id.txt")"
    archive_key="$BACKUP_KEY_DIR/archive/${key_id}.key"
    if [[ -f "$archive_key" ]]; then
      echo "$archive_key"
      return 0
    fi
  fi

  echo "no key available for backup dir $backup_dir" >&2
  return 1
}

verify_backup_secure() {
  local backup_dir="${1:-}"
  if [[ -z "$backup_dir" ]]; then
    echo "missing backup dir: usage scripts/vvtv-runbook.sh verify-backup-secure <backup_dir>" >&2
    exit 1
  fi
  local key_file tmp_dir
  key_file="$(resolve_backup_key_for_dir "$backup_dir")"
  tmp_dir="$(mktemp -d "${TMPDIR:-/tmp}/vvtv-secure-verify.XXXXXX")"

  openssl enc -d -aes-256-cbc -pbkdf2 -in "$backup_dir/backup.tar.gz.enc" -out "$tmp_dir/backup.tar.gz" -pass "file:$key_file"
  mkdir -p "$tmp_dir/backup"
  tar -C "$tmp_dir/backup" -xzf "$tmp_dir/backup.tar.gz"
  (cd "$ROOT_DIR" && cargo run -q -p vvtv-admin -- verify --backup-dir "$tmp_dir/backup")
  rm -rf "$tmp_dir"
}

restore_metadata_secure() {
  local backup_dir="${1:-}"
  if [[ -z "$backup_dir" ]]; then
    echo "missing backup dir: usage scripts/vvtv-runbook.sh restore-metadata-secure <backup_dir>" >&2
    exit 1
  fi
  local key_file tmp_dir
  key_file="$(resolve_backup_key_for_dir "$backup_dir")"
  tmp_dir="$(mktemp -d "${TMPDIR:-/tmp}/vvtv-secure-restore.XXXXXX")"

  openssl enc -d -aes-256-cbc -pbkdf2 -in "$backup_dir/backup.tar.gz.enc" -out "$tmp_dir/backup.tar.gz" -pass "file:$key_file"
  mkdir -p "$tmp_dir/backup"
  tar -C "$tmp_dir/backup" -xzf "$tmp_dir/backup.tar.gz"
  (cd "$ROOT_DIR" && cargo run -q -p vvtv-admin -- restore \
    --backup-dir "$tmp_dir/backup" \
    --state-db "${VVTV_STATE_DB_PATH:-runtime/state/vvtv.db}" \
    --owner-card "${VVTV_OWNER_CARD_PATH:-config/owner_card.sample.yaml}" \
    --force)
  rm -rf "$tmp_dir"
}

latest_export() {
  local export_dir="$ROOT_DIR/runtime/exports"
  if [[ ! -d "$export_dir" ]]; then
    echo "no export directory yet: $export_dir"
    return 1
  fi
  ls -1t "$export_dir"/audit-*.json 2>/dev/null | head -n 1
}

get_state() {
  curl -sS "${API_URL}/v1/status"
}

emergency_toggle() {
  call_signed_control "/v1/control/emergency-mode"
}

emergency_on() {
  local state
  state="$(get_state)"
  if echo "$state" | grep -q '"state":"EMERGENCY"'; then
    echo "$state"
    return 0
  fi
  emergency_toggle
}

emergency_off() {
  local state
  state="$(get_state)"
  if echo "$state" | grep -q '"state":"RUNNING"'; then
    echo "$state"
    return 0
  fi
  emergency_toggle
}

main() {
  require_cmd cargo
  require_cmd curl
  require_cmd openssl
  require_cmd tar

  local cmd="${1:-}"
  case "$cmd" in
    force-nightly)
      force_nightly
      ;;
    export-audits)
      force_nightly
      latest_export
      ;;
    backup-key-rotate)
      backup_key_rotate
      ;;
    backup-metadata)
      backup_metadata
      ;;
    backup-metadata-secure)
      backup_metadata_secure
      ;;
    restore-metadata)
      restore_metadata "${2:-}"
      ;;
    restore-metadata-secure)
      restore_metadata_secure "${2:-}"
      ;;
    verify-backup)
      verify_backup "${2:-}"
      ;;
    verify-backup-secure)
      verify_backup_secure "${2:-}"
      ;;
    emergency-toggle)
      emergency_toggle
      ;;
    emergency-on)
      emergency_on
      ;;
    emergency-off)
      emergency_off
      ;;
    *)
      usage
      exit 1
      ;;
  esac
}

main "$@"
