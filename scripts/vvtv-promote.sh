#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
API_URL="${VVTV_API_URL:-http://127.0.0.1:7070}"
RESULT_FILE_INPUT="${1:-}"
OUT_DIR="${VVTV_PROMOTE_OUT_DIR:-$ROOT_DIR/runtime/releases/promotions}"

mkdir -p "$OUT_DIR"

find_latest_result() {
  ls -1td "$ROOT_DIR"/runtime/canary/*/result.env 2>/dev/null | head -n 1
}

load_kv() {
  local file="$1"
  grep -E '^[a-zA-Z_][a-zA-Z0-9_]*=' "$file"
}

main() {
  local result_file
  if [[ -n "$RESULT_FILE_INPUT" ]]; then
    result_file="$RESULT_FILE_INPUT"
  else
    result_file="$(find_latest_result)"
  fi

  if [[ -z "${result_file:-}" || ! -f "$result_file" ]]; then
    echo "canary result file not found" >&2
    exit 1
  fi

  local canary_status
  canary_status="$(load_kv "$result_file" | sed -n 's/^status=//p' | tail -n 1)"
  if [[ "$canary_status" != "PASS" ]]; then
    echo "canary gate failed: status=$canary_status" >&2
    exit 1
  fi

  local alerts
  alerts="$(curl -sS "$API_URL/v1/alerts")"

  local high_critical
  high_critical="$(echo "$alerts" | awk '
    BEGIN { c = 0 }
    {
      line = $0
      while (match(line, /"severity":"(high|critical)"/)) {
        c++
        line = substr(line, RSTART + RLENGTH)
      }
    }
    END { print c }
  ')"

  if [[ "$high_critical" != "0" ]]; then
    echo "promotion blocked: high/critical alerts active ($high_critical)" >&2
    exit 1
  fi

  local ts out
  ts="$(date -u +%Y%m%dT%H%M%SZ)"
  out="$OUT_DIR/promotion-$ts.env"

  cat >"$out" <<RESULT
status=PROMOTED
promoted_at_utc=$ts
canary_result_file=$result_file
alerts_high_critical=$high_critical
api_url=$API_URL
RESULT

  echo "promotion_record=$out"
}

main "$@"
