#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

STATE_DIR="${VVTV_SMOKE_STATE_DIR:-runtime/smoke}"
STATE_DB="$STATE_DIR/vvtv.db"
HLS_PLAYLIST="runtime/hls/index.m3u8"

mkdir -p "$STATE_DIR"
rm -f "$STATE_DB" "$HLS_PLAYLIST"

printf '==> Running Rust test suite\n'
cargo test

printf '\n==> Running one VVTV orchestrator cycle\n'
VVTV_RUN_ONCE=1 VVTV_STATE_DB="$STATE_DB" cargo run -p vvtv-orchestrator

printf '\n==> Verifying generated HLS playlist\n'
if [[ ! -s "$HLS_PLAYLIST" ]]; then
  printf 'ERROR: expected non-empty playlist at %s\n' "$HLS_PLAYLIST" >&2
  exit 1
fi

printf 'VVTV smoke check passed.\n'
printf 'state_db=%s\n' "$STATE_DB"
printf 'hls_playlist=%s\n' "$HLS_PLAYLIST"
