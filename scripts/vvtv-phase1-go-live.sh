#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TS="$(date -u +%Y%m%dT%H%M%SZ)"
OUT_DIR="${VVTV_PHASE1_OUT_DIR:-$ROOT_DIR/runtime/go-live/phase1/$TS}"
API_URL="${VVTV_API_URL:-http://127.0.0.1:7070}"
API_READY_TIMEOUT_SECS="${VVTV_PHASE1_API_READY_TIMEOUT_SECS:-45}"

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
  echo "[phase1] running $name"
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

main() {
  require_cmd cargo
  require_cmd curl

  run_with_log "cargo-test" cargo test -- --test-threads=1
  run_with_log "orchestrator-run-once" env VVTV_RUN_ONCE=1 cargo run -q -p vvtv-orchestrator

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

  curl -sSf "$API_URL/v1/status" >"$OUT_DIR/status.json"
  curl -sSf "$API_URL/v1/alerts" >"$OUT_DIR/alerts.json"
  curl -sSf "$API_URL/metrics" >"$OUT_DIR/metrics.prom"

  cat >"$OUT_DIR/summary.txt" <<SUMMARY
phase=1
status=PASS
timestamp_utc=$TS
api_url=$API_URL
artifacts_dir=$OUT_DIR
checks=cargo-test,orchestrator-run-once,control-api-status,control-api-alerts,control-api-metrics
SUMMARY

  echo "phase1_status=PASS"
  echo "phase1_artifacts=$OUT_DIR"
}

main "$@"
