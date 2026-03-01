# Release Checklist (Sprint 3)

## Fase 1 (Go Live) — execução consolidada

- [ ] Executar gate automatizado da Fase 1:

```bash
scripts/vvtv-phase1-go-live.sh
```

- [ ] Verificar `runtime/go-live/phase1/<timestamp>/summary.txt` com `status=PASS`
- [ ] Anexar artefatos (`status.json`, `alerts.json`, `metrics.prom`, logs) ao pacote de release

## Build and tests

- [ ] `cargo test` green
- [ ] `cargo run -p vvtv-orchestrator` starts and processes at least one commit window
- [ ] `cargo run -p vvtv-control-api` serves `/v1/status`, `/metrics`, `/v1/alerts`

## Security and secrets

- [ ] `VVTV_ENV=production` configured in runtime environment
- [ ] Non-dev secrets configured (no `dev-token`/`dev-secret`)
- [ ] Cloudflare credentials configured if sync is enabled
- [ ] Control API auth verified with signed request

## Operational readiness

- [ ] `scripts/vvtv-runbook.sh force-nightly` succeeds
- [ ] `scripts/vvtv-runbook.sh export-audits` returns a file path
- [ ] `scripts/vvtv-runbook.sh backup-key-rotate` rotates key and archives old key
- [ ] `scripts/vvtv-runbook.sh backup-key-ensure` enforces max key age policy
- [ ] `scripts/vvtv-runbook.sh backup-key-prune` removes old archived keys
- [ ] `scripts/vvtv-runbook.sh backup-metadata` creates `runtime/backups/<timestamp>/manifest.json`
- [ ] `scripts/vvtv-runbook.sh backup-metadata-secure` creates encrypted payload
- [ ] `scripts/vvtv-runbook.sh verify-backup <backup_dir>` validates checksums
- [ ] `scripts/vvtv-runbook.sh verify-backup-secure <backup_dir>` validates encrypted backup
- [ ] `scripts/vvtv-runbook.sh restore-metadata <backup_dir>` succeeds in maintenance window
- [ ] `scripts/vvtv-runbook.sh restore-metadata-secure <backup_dir>` succeeds in maintenance window
- [ ] `scripts/vvtv-runbook.sh emergency-on` and `emergency-off` both succeed

## Soak run

- [ ] Execute 24h soak:

```bash
VVTV_SOAK_HOURS=24 VVTV_SOAK_INTERVAL_SECS=60 scripts/vvtv-soak.sh
```

- [ ] Verify `runtime/soak/<timestamp>/summary.txt` with `verdict=PASS`
- [ ] Attach soak `summary.txt`, `status-samples.log`, `metrics-samples.log`, `alerts-samples.log` to release artifact

## Canary gate

- [ ] Execute canary with automatic rollback enabled:

```bash
VVTV_CANARY_SOAK_HOURS=1 scripts/vvtv-canary.sh
```

- [ ] Verify `runtime/canary/<timestamp>/result.env` has `status=PASS`
- [ ] Optional: `VVTV_CANARY_AUTO_PROMOTE=1 scripts/vvtv-canary.sh` ends with `promotion_status=PROMOTED`

## Promotion gate

- [ ] Start control API and verify alerts endpoint is reachable
- [ ] Execute promotion gate:

```bash
scripts/vvtv-promote.sh runtime/canary/<timestamp>/result.env
```

- [ ] Verify promotion record in `runtime/releases/promotions/promotion-<ts>.env`

## Cloudflare integration

- [ ] D1 migration applied (`cloudflare/worker/migrations/0001_init.sql`)
- [ ] Worker deployed
- [ ] Orchestrator cloud sync variables configured
- [ ] Confirm Worker serves latest daily/weekly reports from D1

## Go/No-Go criteria

Go if all items above pass and no critical/high alert remains active.
No-Go if any soak failure, auth failure, or stream continuity regression is observed.
