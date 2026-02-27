# Testing the MVP for real (including Cloudflare)

This guide gets you from a clean clone to a verified run: one orchestrator cycle and, if configured, Cloudflare sync.

## Prerequisites

- **Rust** stable (see root `Cargo.toml` for `rust-version`)
- **ffmpeg** and **ffprobe** on `PATH` (used by `vvtv-prep` for QA)
- **SQLite** (embedded via `rusqlite`; no separate install needed)

## 1. Build and unit tests

```bash
cargo test
```

All workspace tests (including pipeline integration tests) should pass.

## 2. Run one orchestrator cycle (no Cloudflare)

Uses mock discovery and the sample OwnerCard. No env vars required in `VVTV_ENV=dev`.

```bash
VVTV_RUN_ONCE=1 cargo run -p vvtv-orchestrator
```

You should see:

- Discovery window → plans saved
- Commit window → fetch/prep/queue/stream (HLS output under `runtime/hls/`)
- JSON metrics printed at the end
- Process exits after one cycle

State is written to `runtime/state/vvtv.db`.

## 3. Run the Control API (local)

In another terminal:

```bash
cargo run -p vvtv-control-api
```

Defaults: listen on `0.0.0.0:7070`, `VVTV_STATE_DB=runtime/state/vvtv.db`.

- **Status:** `curl http://127.0.0.1:7070/v1/status`
- **Metrics:** `curl http://127.0.0.1:7070/metrics`
- **Alerts:** `curl http://127.0.0.1:7070/v1/alerts`

Control endpoints (`/v1/control/*`) require signed requests; see README “Control-plane auth”.

## 4. Cloudflare Worker (production API)

The Worker at **https://api.voulezvous.tv** provides:

- **Public (no auth):** `GET /v1/status`, `GET /v1/reports/daily?date=YYYY-MM-DD`, `GET /v1/reports/weekly?week=YYYY-Www`
- **Ingest (auth):** `POST /v1/ingest/status`, `POST /v1/ingest/daily`, `POST /v1/ingest/weekly` (Bearer + HMAC)

Deploy and secrets:

```bash
cd cloudflare/worker
npx wrangler d1 execute vvtv-control --file migrations/0001_init.sql --remote
npx wrangler secret put CONTROL_TOKEN
npx wrangler secret put CONTROL_SECRET
npx wrangler deploy
```

Use the same `CONTROL_TOKEN` and `CONTROL_SECRET` in the LAB (see below).

## 5. Run one cycle **with** Cloudflare sync

The orchestrator pushes status after each commit window and daily/weekly reports during nightly. To test sync:

1. **Set credentials** (must match Worker secrets):

   ```bash
   export VVTV_CLOUDFLARE_BASE_URL="https://api.voulezvous.tv"
   export VVTV_CLOUDFLARE_TOKEN="<same as CONTROL_TOKEN in Worker>"
   export VVTV_CLOUDFLARE_SECRET="<same as CONTROL_SECRET in Worker>"
   ```

   Or add the same three variables to `.env` and source it: `set -a && source .env && set +a`.

2. **Run one cycle:**

   ```bash
   VVTV_RUN_ONCE=1 cargo run -p vvtv-orchestrator
   ```

   Look for log lines: `cloudflare-status-sync-ok` (after commit) or `cloudflare-status-sync-failed` (e.g. wrong token/secret or network).

3. **Verify status on Cloudflare:**

   ```bash
   curl -s https://api.voulezvous.tv/v1/status | jq .
   ```

   You should see the snapshot payload (e.g. `buffer_minutes`, `plans_committed`, `qa_pass_rate`) instead of the default `{ "state": "RUNNING", "buffer_minutes": 60, "source": "default" }`.

## 6. Optional: nightly → daily/weekly reports

To push daily and weekly reports without waiting until 03:00:

```bash
VVTV_RUN_ONCE=1 VVTV_FORCE_NIGHTLY=1 cargo run -p vvtv-orchestrator
```

Then:

- `curl "https://api.voulezvous.tv/v1/reports/daily?date=$(date +%Y-%m-%d)"`
- `curl "https://api.voulezvous.tv/v1/reports/weekly?week=$(date +%G-W%V)"`

## 7. Quick script

Use the helper script to run one cycle and, if Cloudflare is configured, check `/v1/status`:

```bash
# Optional: put VVTV_CLOUDFLARE_BASE_URL, VVTV_CLOUDFLARE_TOKEN, VVTV_CLOUDFLARE_SECRET in .env
./scripts/test-cloudflare-sync.sh
```

## Troubleshooting

| Symptom | Check |
|--------|--------|
| `cloudflare-status-sync-failed` | Token/secret match Worker; `VVTV_CLOUDFLARE_BASE_URL` has no trailing slash; network/firewall to api.voulezvous.tv |
| `invalid signature` from Worker | Same secret on both sides; body and path match (Worker uses pathname; agent uses path like `/v1/ingest/status`) |
| `stale timestamp` | Clocks in sync (Worker allows ±5 min) |
| `/v1/status` still shows default | No successful ingest yet; run one cycle with sync env set and look for `cloudflare-status-sync-ok` |

## Note on `.env`

Your `.env` may contain `CLOUDFLARE_DNS_TOKEN` (e.g. for DNS automation). That is **not** used by the orchestrator. For sync you need:

- `VVTV_CLOUDFLARE_BASE_URL`
- `VVTV_CLOUDFLARE_TOKEN`
- `VVTV_CLOUDFLARE_SECRET`

Add these to `.env` (and keep `.env` out of version control) if you want to source them for real runs.
