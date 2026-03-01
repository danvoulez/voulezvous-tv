#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TS="$(date -u +%Y%m%dT%H%M%SZ)"
OUT_DIR="${VVTV_PHASE3_OUT_DIR:-$ROOT_DIR/runtime/go-live/phase3/$TS}"
CANARY_SOAK_HOURS="${VVTV_PHASE3_CANARY_SOAK_HOURS:-0}"
CANARY_SOAK_INTERVAL_SECS="${VVTV_PHASE3_CANARY_SOAK_INTERVAL_SECS:-10}"
CANARY_SOAK_MAX_SAMPLES="${VVTV_PHASE3_CANARY_SOAK_MAX_SAMPLES:-3}"
AUTO_PROMOTE="${VVTV_PHASE3_AUTO_PROMOTE:-1}"
PROMOTE_STRICT="${VVTV_PHASE3_PROMOTE_STRICT:-1}"
CHECK_CLOUDFLARE="${VVTV_PHASE3_CHECK_CLOUDFLARE:-0}"

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
  echo "[phase3] running $name"
  (
    cd "$ROOT_DIR"
    "$@"
  ) >"$log" 2>&1
}

extract_from_result() {
  local key="$1"
  local file="$2"
  sed -n "s/^${key}=//p" "$file" | tail -n 1
}

main() {
  require_cmd sed

  run_with_log "canary" env \
    VVTV_CANARY_OUT_DIR="$OUT_DIR/canary" \
    VVTV_CANARY_SOAK_HOURS="$CANARY_SOAK_HOURS" \
    VVTV_CANARY_SOAK_INTERVAL_SECS="$CANARY_SOAK_INTERVAL_SECS" \
    VVTV_CANARY_SOAK_MAX_SAMPLES="$CANARY_SOAK_MAX_SAMPLES" \
    VVTV_CANARY_AUTO_PROMOTE="$AUTO_PROMOTE" \
    VVTV_CANARY_PROMOTE_STRICT="$PROMOTE_STRICT" \
    scripts/vvtv-canary.sh

  local result_file="$OUT_DIR/canary/result.env"
  if [[ ! -f "$result_file" ]]; then
    echo "missing canary result file: $result_file" >&2
    exit 1
  fi

  local canary_status promotion_status promotion_record
  canary_status="$(extract_from_result "status" "$result_file")"
  promotion_status="$(extract_from_result "promotion_status" "$result_file")"
  promotion_record="$(extract_from_result "promotion_record" "$result_file")"

  if [[ "$canary_status" != "PASS" ]]; then
    echo "canary returned non-pass status: $canary_status" >&2
    exit 1
  fi

  if [[ "$AUTO_PROMOTE" == "1" && "$promotion_status" != "PROMOTED" ]]; then
    echo "promotion gate did not promote release: $promotion_status" >&2
    exit 1
  fi

  local checks="canary"
  if [[ "$CHECK_CLOUDFLARE" == "1" ]]; then
    run_with_log "cloudflare-sync-check" scripts/test-cloudflare-sync.sh
    checks="$checks,cloudflare-sync-check"
  fi

  cat >"$OUT_DIR/summary.txt" <<SUMMARY
phase=3
status=PASS
timestamp_utc=$TS
artifacts_dir=$OUT_DIR
canary_result_file=$result_file
canary_status=$canary_status
promotion_status=$promotion_status
promotion_record=$promotion_record
checks=$checks
SUMMARY

  echo "phase3_status=PASS"
  echo "phase3_artifacts=$OUT_DIR"
}

main "$@"
