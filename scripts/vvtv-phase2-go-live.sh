#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TS="$(date -u +%Y%m%dT%H%M%SZ)"
OUT_DIR="${VVTV_PHASE2_OUT_DIR:-$ROOT_DIR/runtime/go-live/phase2/$TS}"
API_URL="${VVTV_API_URL:-http://127.0.0.1:7070}"
API_READY_TIMEOUT_SECS="${VVTV_PHASE2_API_READY_TIMEOUT_SECS:-45}"
SOAK_HOURS="${VVTV_PHASE2_SOAK_HOURS:-0}"
SOAK_INTERVAL_SECS="${VVTV_PHASE2_SOAK_INTERVAL_SECS:-10}"
SOAK_MAX_SAMPLES="${VVTV_PHASE2_SOAK_MAX_SAMPLES:-3}"

mkdir -p "$OUT_DIR"

require_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "missing required command: $1" >&2
    exit 1
  fi
}

run_with_log() {
  local name="$1"
  shift
  local log="$OUT_DIR/$name.log"
  echo "[phase2] running $name"
  (
    cd "$ROOT_DIR"
    "$@"
  ) >"$log" 2>&1
}

wait_api_ready() {
  local elapsed=0
  while (( elapsed < API_READY_TIMEOUT_SECS )); do
    if curl -sSf "$API_URL/v1/status" >/dev/null 2>&1; then
      return 0
    fi
    sleep 1
    elapsed=$((elapsed + 1))
  done
  return 1
}

extract_kv_from_log() {
  local key="$1"
  local file="$2"
  sed -n "s/^${key}=//p" "$file" | tail -n 1
}

main() {
  require_cmd cargo
  require_cmd curl
  require_cmd sed

  run_with_log "force-nightly" scripts/vvtv-runbook.sh force-nightly
  run_with_log "export-audits" scripts/vvtv-runbook.sh export-audits
  run_with_log "backup-key-rotate" scripts/vvtv-runbook.sh backup-key-rotate
  run_with_log "backup-key-ensure" scripts/vvtv-runbook.sh backup-key-ensure
  run_with_log "backup-key-prune" scripts/vvtv-runbook.sh backup-key-prune

  run_with_log "backup-metadata" scripts/vvtv-runbook.sh backup-metadata
  local backup_dir
  backup_dir="$(extract_kv_from_log "backup_dir" "$OUT_DIR/backup-metadata.log")"
  if [[ -z "$backup_dir" ]]; then
    echo "failed to resolve backup_dir from $OUT_DIR/backup-metadata.log" >&2
    exit 1
  fi

  run_with_log "verify-backup" scripts/vvtv-runbook.sh verify-backup "$backup_dir"
  run_with_log "restore-metadata" scripts/vvtv-runbook.sh restore-metadata "$backup_dir"

  run_with_log "backup-metadata-secure" scripts/vvtv-runbook.sh backup-metadata-secure
  local secure_backup_dir
  secure_backup_dir="$(extract_kv_from_log "secure_backup_dir" "$OUT_DIR/backup-metadata-secure.log")"
  if [[ -z "$secure_backup_dir" ]]; then
    echo "failed to resolve secure_backup_dir from $OUT_DIR/backup-metadata-secure.log" >&2
    exit 1
  fi

  run_with_log "verify-backup-secure" scripts/vvtv-runbook.sh verify-backup-secure "$secure_backup_dir"
  run_with_log "restore-metadata-secure" scripts/vvtv-runbook.sh restore-metadata-secure "$secure_backup_dir"

  local api_log="$OUT_DIR/control-api.log"
  (
    cd "$ROOT_DIR"
    cargo run -q -p vvtv-control-api
  ) >"$api_log" 2>&1 &
  local api_pid="$!"

  cleanup() {
    if [[ -n "${api_pid:-}" ]]; then
      kill "$api_pid" 2>/dev/null || true
      wait "$api_pid" 2>/dev/null || true
    fi
  }
  trap cleanup EXIT

  if ! wait_api_ready; then
    echo "control API did not become ready at $API_URL within ${API_READY_TIMEOUT_SECS}s" >&2
    exit 1
  fi

  run_with_log "emergency-on" scripts/vvtv-runbook.sh emergency-on
  run_with_log "emergency-off" scripts/vvtv-runbook.sh emergency-off

  run_with_log "soak" env \
    VVTV_SOAK_HOURS="$SOAK_HOURS" \
    VVTV_SOAK_INTERVAL_SECS="$SOAK_INTERVAL_SECS" \
    VVTV_SOAK_MAX_SAMPLES="$SOAK_MAX_SAMPLES" \
    VVTV_SOAK_OUT_DIR="$OUT_DIR/soak" \
    scripts/vvtv-soak.sh

  cat >"$OUT_DIR/summary.txt" <<SUMMARY
phase=2
status=PASS
timestamp_utc=$TS
api_url=$API_URL
artifacts_dir=$OUT_DIR
backup_dir=$backup_dir
secure_backup_dir=$secure_backup_dir
soak_summary=$OUT_DIR/soak/summary.txt
checks=force-nightly,export-audits,backup-key-rotate,backup-key-ensure,backup-key-prune,backup-metadata,verify-backup,restore-metadata,backup-metadata-secure,verify-backup-secure,restore-metadata-secure,emergency-on,emergency-off,soak
SUMMARY

  echo "phase2_status=PASS"
  echo "phase2_artifacts=$OUT_DIR"
}

main "$@"
