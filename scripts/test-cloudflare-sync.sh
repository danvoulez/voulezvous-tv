#!/usr/bin/env bash
# Run one orchestrator cycle and, if Cloudflare is configured, verify status on the public API.
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

if [[ -f .env ]]; then
  set -a
  source .env
  set +a
fi

echo "=== Running one orchestrator cycle (VVTV_RUN_ONCE=1) ==="
VVTV_RUN_ONCE=1 cargo run -p vvtv-orchestrator

BASE="${VVTV_CLOUDFLARE_BASE_URL:-}"
if [[ -z "$BASE" ]]; then
  echo "=== VVTV_CLOUDFLARE_BASE_URL not set; skipping Cloudflare status check ==="
  echo "To test sync: set VVTV_CLOUDFLARE_BASE_URL, VVTV_CLOUDFLARE_TOKEN, VVTV_CLOUDFLARE_SECRET (e.g. in .env) and re-run."
  exit 0
fi

# Trim trailing slash
BASE="${BASE%/}"
echo ""
echo "=== Fetching $BASE/v1/status ==="
curl -sS "$BASE/v1/status" | jq . 2>/dev/null || curl -sS "$BASE/v1/status"
