# Cloudflare Integration

## Worker + D1 setup

1. Configure `cloudflare/worker/wrangler.toml` with real D1 `database_id` and route.
2. Apply migration:

```bash
cd cloudflare/worker
npx wrangler d1 execute vvtv-control --file migrations/0001_init.sql --remote
```

3. Set Worker secrets:

```bash
npx wrangler secret put CONTROL_TOKEN
npx wrangler secret put CONTROL_SECRET
```

4. Deploy:

```bash
npx wrangler deploy
```

Current production route:

- `https://voulezvous.tv/vvtv/*`
- health read: `GET https://voulezvous.tv/vvtv/v1/status`

## LAB orchestrator -> Cloud sync

Set environment in the LAB process:

- `VVTV_CLOUDFLARE_BASE_URL=https://voulezvous.tv/vvtv`
- `VVTV_CLOUDFLARE_TOKEN=<token>`
- `VVTV_CLOUDFLARE_SECRET=<secret>`

When configured, orchestrator publishes:

- status snapshot after commit windows (`POST /v1/ingest/status`)
- daily report in nightly (`POST /v1/ingest/daily`)
- weekly report in nightly (`POST /v1/ingest/weekly`)

Control-agent resilience defaults:

- retries: 3
- exponential backoff base: 300ms
- circuit breaker opens after 3 transient failures
- circuit cooldown: 30s
