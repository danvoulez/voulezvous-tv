#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
API_URL="${VVTV_API_URL:-http://127.0.0.1:7070}"
CONTROL_TOKEN="${VVTV_CONTROL_TOKEN:-dev-token}"
CONTROL_SECRET="${VVTV_CONTROL_SECRET:-dev-secret}"

usage() {
  cat <<USAGE
Usage: scripts/vvtv-runbook.sh <command>

Commands:
  force-nightly      Run orchestrator once with forced nightly maintenance
  export-audits      Force nightly and print latest audit export file
  backup-metadata    Snapshot state.db + OwnerCard with manifest checksum
  restore-metadata   Restore state.db + OwnerCard from backup dir
  emergency-toggle   Toggle emergency mode via signed control API
  emergency-on       Ensure emergency mode is ON
  emergency-off      Ensure emergency mode is OFF
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

  local cmd="${1:-}"
  case "$cmd" in
    force-nightly)
      force_nightly
      ;;
    export-audits)
      force_nightly
      latest_export
      ;;
    backup-metadata)
      backup_metadata
      ;;
    restore-metadata)
      restore_metadata "${2:-}"
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
