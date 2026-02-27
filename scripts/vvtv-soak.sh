#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SOAK_HOURS="${VVTV_SOAK_HOURS:-24}"
CHECK_INTERVAL_SECS="${VVTV_SOAK_INTERVAL_SECS:-60}"
MAX_SAMPLES="${VVTV_SOAK_MAX_SAMPLES:-0}"
API_URL="${VVTV_API_URL:-http://127.0.0.1:7070}"
OUT_DIR="${VVTV_SOAK_OUT_DIR:-$ROOT_DIR/runtime/soak/$(date +%Y%m%d-%H%M%S)}"
RUN_ORCH="${VVTV_SOAK_RUN_ORCH:-1}"
RUN_API="${VVTV_SOAK_RUN_API:-1}"
BOOT_TIMEOUT_SECS="${VVTV_SOAK_BOOT_TIMEOUT_SECS:-90}"

mkdir -p "$OUT_DIR"

ORCH_PID=""
API_PID=""
SAMPLES=0
FAIL_STATUS=0
FAIL_METRICS=0
FAIL_ALERTS=0
BUFFER_CRITICAL_COUNT=0
STREAM_DISRUPTIONS_NONZERO=0

cleanup() {
  if [[ -n "$ORCH_PID" ]]; then
    kill "$ORCH_PID" 2>/dev/null || true
  fi
  if [[ -n "$API_PID" ]]; then
    kill "$API_PID" 2>/dev/null || true
  fi
}
trap cleanup EXIT

start_processes() {
  if [[ "$RUN_API" == "1" ]]; then
    (
      cd "$ROOT_DIR"
      VVTV_CONTROL_TOKEN="${VVTV_CONTROL_TOKEN:-dev-token}" \
      VVTV_CONTROL_SECRET="${VVTV_CONTROL_SECRET:-dev-secret}" \
      cargo run -q -p vvtv-control-api
    ) >"$OUT_DIR/control-api.log" 2>&1 &
    API_PID="$!"
  fi

  if [[ "$RUN_ORCH" == "1" ]]; then
    (
      cd "$ROOT_DIR"
      cargo run -q -p vvtv-orchestrator
    ) >"$OUT_DIR/orchestrator.log" 2>&1 &
    ORCH_PID="$!"
  fi

  wait_api_ready
}

require_tools() {
  command -v curl >/dev/null 2>&1 || { echo "missing curl" >&2; exit 1; }
}

wait_api_ready() {
  if [[ "$RUN_API" != "1" ]]; then
    return 0
  fi

  local elapsed=0
  while (( elapsed < BOOT_TIMEOUT_SECS )); do
    if curl -sSf "$API_URL/v1/status" >/dev/null 2>&1; then
      return 0
    fi
    sleep 1
    elapsed=$((elapsed + 1))
  done

  echo "control API did not become ready within ${BOOT_TIMEOUT_SECS}s" >&2
  return 1
}

extract_json_num() {
  local key="$1"
  sed -n "s/.*\"${key}\":\([0-9.\-]*\).*/\1/p" | head -n 1
}

extract_json_str() {
  local key="$1"
  sed -n "s/.*\"${key}\":\"\([^\"]*\)\".*/\1/p" | head -n 1
}

run_checks_once() {
  local ts status metrics alerts buffer disruptions
  ts="$(date -u +%Y-%m-%dT%H:%M:%SZ)"

  status="$(curl -s "$API_URL/v1/status" || true)"
  metrics="$(curl -s "$API_URL/metrics" || true)"
  alerts="$(curl -s "$API_URL/v1/alerts" || true)"

  echo "$ts | status=$status" >>"$OUT_DIR/status-samples.log"
  echo "$ts | alerts=$alerts" >>"$OUT_DIR/alerts-samples.log"
  printf "%s\n%s\n" "# --- $ts ---" "$metrics" >>"$OUT_DIR/metrics-samples.log"

  if [[ -z "$status" ]]; then
    FAIL_STATUS=$((FAIL_STATUS + 1))
  fi
  if [[ -z "$metrics" ]]; then
    FAIL_METRICS=$((FAIL_METRICS + 1))
  fi
  if [[ -z "$alerts" ]]; then
    FAIL_ALERTS=$((FAIL_ALERTS + 1))
  fi

  buffer="$(echo "$metrics" | sed -n 's/^vvtv_buffer_minutes \([0-9.\-]*\)$/\1/p' | head -n 1)"
  disruptions="$(echo "$metrics" | sed -n 's/^vvtv_stream_disruptions \([0-9.\-]*\)$/\1/p' | head -n 1)"

  if [[ -n "$buffer" ]]; then
    awk "BEGIN { exit !($buffer < 20) }" && BUFFER_CRITICAL_COUNT=$((BUFFER_CRITICAL_COUNT + 1)) || true
  fi

  if [[ -n "$disruptions" ]]; then
    awk "BEGIN { exit !($disruptions > 0) }" && STREAM_DISRUPTIONS_NONZERO=$((STREAM_DISRUPTIONS_NONZERO + 1)) || true
  fi

  SAMPLES=$((SAMPLES + 1))
}

write_summary() {
  local verdict="PASS"

  if (( FAIL_STATUS > 0 || FAIL_METRICS > 0 || FAIL_ALERTS > 0 )); then
    verdict="FAIL"
  fi
  if (( BUFFER_CRITICAL_COUNT > 0 )); then
    verdict="FAIL"
  fi

  cat >"$OUT_DIR/summary.txt" <<SUMMARY
VVTV Soak Summary
verdict=${verdict}
samples=${SAMPLES}
fail_status_requests=${FAIL_STATUS}
fail_metrics_requests=${FAIL_METRICS}
fail_alerts_requests=${FAIL_ALERTS}
buffer_critical_observations=${BUFFER_CRITICAL_COUNT}
stream_disruptions_nonzero_observations=${STREAM_DISRUPTIONS_NONZERO}
soak_hours=${SOAK_HOURS}
check_interval_secs=${CHECK_INTERVAL_SECS}
SUMMARY

  cat "$OUT_DIR/summary.txt"

  if [[ "$verdict" != "PASS" ]]; then
    return 1
  fi
}

main() {
  require_tools
  start_processes

  local total_secs=$((SOAK_HOURS * 3600))
  local elapsed=0

  while (( elapsed < total_secs )); do
    if (( MAX_SAMPLES > 0 && SAMPLES >= MAX_SAMPLES )); then
      break
    fi
    run_checks_once
    sleep "$CHECK_INTERVAL_SECS"
    elapsed=$((elapsed + CHECK_INTERVAL_SECS))
  done

  if (( total_secs == 0 && MAX_SAMPLES > 0 )); then
    while (( SAMPLES < MAX_SAMPLES )); do
      run_checks_once
      sleep "$CHECK_INTERVAL_SECS"
    done
  fi

  write_summary
}

main "$@"
