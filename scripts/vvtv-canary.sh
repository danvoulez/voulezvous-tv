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
AUTO_PROMOTE="${VVTV_CANARY_AUTO_PROMOTE:-0}"
PROMOTE_STRICT="${VVTV_CANARY_PROMOTE_STRICT:-1}"
PROMOTE_BOOTSTRAP_API="${VVTV_CANARY_PROMOTE_BOOTSTRAP_API:-1}"
API_URL="${VVTV_API_URL:-http://127.0.0.1:7070}"
PROMOTE_BOOT_TIMEOUT_SECS="${VVTV_CANARY_PROMOTE_BOOT_TIMEOUT_SECS:-90}"

PROMOTION_STATUS="SKIPPED"
PROMOTION_RECORD=""

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
  local quiet="${4:-0}"

  cat >"$RESULT_FILE" <<RESULT
status=$status
backup_dir=$backup_dir
rollback_applied=$rollback_applied
canary_out_dir=$OUT_DIR
soak_summary=$SOAK_OUT_DIR/summary.txt
promotion_status=$PROMOTION_STATUS
promotion_record=$PROMOTION_RECORD
RESULT

  if [[ "$quiet" != "1" ]]; then
    cat "$RESULT_FILE"
  fi
}

wait_api_ready() {
  local elapsed=0
  while (( elapsed < PROMOTE_BOOT_TIMEOUT_SECS )); do
    if curl -sSf "$API_URL/v1/status" >/dev/null 2>&1; then
      return 0
    fi
    sleep 1
    elapsed=$((elapsed + 1))
  done
  return 1
}

run_promotion_gate() {
  local result_file="$1"
  local out rc api_pid
  api_pid=""

  if [[ "$PROMOTE_BOOTSTRAP_API" == "1" ]] && ! curl -sSf "$API_URL/v1/status" >/dev/null 2>&1; then
    (
      cd "$ROOT_DIR"
      cargo run -q -p vvtv-control-api
    ) >"$OUT_DIR/promote-control-api.log" 2>&1 &
    api_pid="$!"
    if ! wait_api_ready; then
      echo "failed to bootstrap control API for promotion gate" >&2
      if [[ -n "$api_pid" ]]; then
        kill "$api_pid" 2>/dev/null || true
      fi
      return 1
    fi
  fi

  set +e
  out="$(
    cd "$ROOT_DIR"
    scripts/vvtv-promote.sh "$result_file" 2>&1
  )"
  rc=$?
  set -e
  if [[ -n "$api_pid" ]]; then
    kill "$api_pid" 2>/dev/null || true
    wait "$api_pid" 2>/dev/null || true
  fi
  if [[ "$rc" -ne 0 ]]; then
    PROMOTION_STATUS="BLOCKED"
    PROMOTION_RECORD=""
    if [[ "$PROMOTE_STRICT" == "1" ]]; then
      echo "$out" >&2
      echo "promotion gate failed in strict mode" >&2
      return "$rc"
    fi
    return 0
  fi

  PROMOTION_STATUS="PROMOTED"
  PROMOTION_RECORD="$(echo "$out" | sed -n 's/^promotion_record=//p' | tail -n 1)"
  return 0
}

main() {
  require_cmd bash
  require_cmd curl
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
    write_result "PASS" "$backup_dir" "0" "1"
    if [[ "$AUTO_PROMOTE" == "1" ]]; then
      run_promotion_gate "$RESULT_FILE"
      write_result "PASS" "$backup_dir" "0"
    else
      write_result "PASS" "$backup_dir" "0"
    fi
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
