#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TS="$(date +%Y%m%d-%H%M%S)"
OUT_DIR="${VVTV_CANARY_OUT_DIR:-$ROOT_DIR/runtime/canary/$TS}"
SOAK_OUT_DIR="$OUT_DIR/soak"
BACKUP_LOG="$OUT_DIR/backup.log"
RESULT_FILE="$OUT_DIR/result.env"

CANARY_SOAK_HOURS="${VVTV_CANARY_SOAK_HOURS:-1}"
CANARY_SOAK_INTERVAL_SECS="${VVTV_CANARY_SOAK_INTERVAL_SECS:-60}"
CANARY_SOAK_MAX_SAMPLES="${VVTV_CANARY_SOAK_MAX_SAMPLES:-0}"
AUTO_ROLLBACK="${VVTV_CANARY_AUTO_ROLLBACK:-1}"

mkdir -p "$OUT_DIR"

require_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "missing required command: $1" >&2
    exit 1
  fi
}

run_backup() {
  (
    cd "$ROOT_DIR"
    scripts/vvtv-runbook.sh backup-metadata
  ) | tee "$BACKUP_LOG"
}

extract_backup_dir() {
  grep '^backup_dir=' "$BACKUP_LOG" | tail -n 1 | cut -d'=' -f2-
}

verify_backup() {
  local backup_dir="$1"
  (
    cd "$ROOT_DIR"
    scripts/vvtv-runbook.sh verify-backup "$backup_dir"
  )
}

run_soak() {
  (
    cd "$ROOT_DIR"
    VVTV_SOAK_OUT_DIR="$SOAK_OUT_DIR" \
    VVTV_SOAK_HOURS="$CANARY_SOAK_HOURS" \
    VVTV_SOAK_INTERVAL_SECS="$CANARY_SOAK_INTERVAL_SECS" \
    VVTV_SOAK_MAX_SAMPLES="$CANARY_SOAK_MAX_SAMPLES" \
    scripts/vvtv-soak.sh
  )
}

rollback() {
  local backup_dir="$1"
  (
    cd "$ROOT_DIR"
    scripts/vvtv-runbook.sh restore-metadata "$backup_dir"
  )
}

write_result() {
  local status="$1"
  local backup_dir="$2"
  local rollback_applied="$3"

  cat >"$RESULT_FILE" <<RESULT
status=$status
backup_dir=$backup_dir
rollback_applied=$rollback_applied
canary_out_dir=$OUT_DIR
soak_summary=$SOAK_OUT_DIR/summary.txt
RESULT

  cat "$RESULT_FILE"
}

main() {
  require_cmd bash
  require_cmd grep
  require_cmd tee

  run_backup

  local backup_dir
  backup_dir="$(extract_backup_dir)"
  if [[ -z "$backup_dir" ]]; then
    echo "failed to extract backup_dir from $BACKUP_LOG" >&2
    exit 1
  fi

  verify_backup "$backup_dir"

  if run_soak; then
    write_result "PASS" "$backup_dir" "0"
    return 0
  fi

  if [[ "$AUTO_ROLLBACK" == "1" ]]; then
    verify_backup "$backup_dir"
    rollback "$backup_dir"
    write_result "FAIL_ROLLBACK_APPLIED" "$backup_dir" "1"
  else
    write_result "FAIL_ROLLBACK_SKIPPED" "$backup_dir" "0"
  fi

  return 1
}

main "$@"
