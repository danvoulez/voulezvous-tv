# VVTV Codex MVP (Rust-first)

Implementacao do blueprint VVTV com pipeline 24/7 mockado/real hibrido ponta a ponta e contratos estaveis.

## Quickstart

```bash
cargo test
cargo run -p vvtv-orchestrator
cargo run -p vvtv-control-api
```

Ambiente:
- `VVTV_ENV=dev` (default)
- Em `VVTV_ENV != dev`, o sistema aplica fail-fast para segredos obrigatorios.

## Control-plane auth

- Endpoints `/v1/control/*` exigem:
  - `Authorization: Bearer <token>`
  - `x-vvtv-ts: <unix_timestamp>`
  - `x-vvtv-signature: <hex_hmac_sha256>`
- Variaveis da API:
  - `VVTV_CONTROL_TOKEN` (default: `dev-token`)
  - `VVTV_CONTROL_SECRET` (default: `dev-secret`)
  - em `VVTV_ENV != dev`, os defaults sao proibidos
- Canonical string assinada:
  - `METHOD + \"\\n\" + PATH + \"\\n\" + TIMESTAMP + \"\\n\" + BODY`

## Scheduler continuo

- O orquestrador roda em loop continuo e executa:
  - janela de descoberta (hora a hora)
  - janela de commit T-4h no intervalo definido no OwnerCard (`commit_interval_minutes`)
  - rotina nightly as 03:00 (hora local)
- Para rodar um unico ciclo e sair:

```bash
VVTV_RUN_ONCE=1 cargo run -p vvtv-orchestrator
```

- Para forcar a rotina nightly (export + retention) fora das 03:00:

```bash
VVTV_RUN_ONCE=1 VVTV_FORCE_NIGHTLY=1 cargo run -p vvtv-orchestrator
```

## Observabilidade

- Endpoint Prometheus: `GET /metrics`
- Alertas operacionais: `GET /v1/alerts`
- A API le estado de `VVTV_STATE_DB` (default `runtime/state/vvtv.db`)

Variaveis de threshold de alerta:
- `VVTV_ALERT_QA_MIN` (default `0.85`)
- `VVTV_ALERT_FALLBACK_ABS` (default `0.30`)
- `VVTV_ALERT_FALLBACK_GROWTH` (default `0.15`)
- `VVTV_ALERT_DISCOVERY_FAIL_COUNT` (default `3`)

Webhook de alertas (opcional):
- `VVTV_ALERT_WEBHOOK_URL`
- `VVTV_ALERT_COOLDOWN_SECS` (default `900`)

Com webhook ativo:
- envia alerta apenas para severidade `high`/`critical`
- dedupe por `code` com cooldown
- envia evento `alert_clear` quando o alerta some

## Cloudflare sync (opcional)

- `VVTV_CLOUDFLARE_BASE_URL`
- `VVTV_CLOUDFLARE_TOKEN`
- `VVTV_CLOUDFLARE_SECRET`
- em `VVTV_ENV != dev`, `VVTV_CLOUDFLARE_BASE_URL` e obrigatoria e, se configurada, token/secret tambem

Valor atual de `VVTV_CLOUDFLARE_BASE_URL` em producao:
- `https://api.voulezvous.tv`

Com essas variaveis, o orquestrador publica snapshots/relatorios para o Worker.
Setup completo: `docs/cloudflare-integration.md`.

## Runbook automation

Script operacional: `scripts/vvtv-runbook.sh`

Exemplos:
- `scripts/vvtv-runbook.sh backup-key-rotate`
- `scripts/vvtv-runbook.sh backup-key-ensure`
- `scripts/vvtv-runbook.sh backup-key-prune`
- `scripts/vvtv-runbook.sh force-nightly`
- `scripts/vvtv-runbook.sh export-audits`
- `scripts/vvtv-runbook.sh backup-metadata`
- `scripts/vvtv-runbook.sh backup-metadata-secure`
- `scripts/vvtv-runbook.sh verify-backup runtime/backups/<timestamp>`
- `scripts/vvtv-runbook.sh verify-backup-secure runtime/backups/<timestamp>`
- `scripts/vvtv-runbook.sh restore-metadata runtime/backups/<timestamp>`
- `scripts/vvtv-runbook.sh restore-metadata-secure runtime/backups/<timestamp>`
- `scripts/vvtv-runbook.sh emergency-on`
- `scripts/vvtv-runbook.sh emergency-off`

## Soak run

Script de soak: `scripts/vvtv-soak.sh`

Exemplo (24h):

```bash
VVTV_SOAK_HOURS=24 VVTV_SOAK_INTERVAL_SECS=60 scripts/vvtv-soak.sh
```

Dry run curto:

```bash
VVTV_SOAK_HOURS=0 VVTV_SOAK_MAX_SAMPLES=3 VVTV_SOAK_INTERVAL_SECS=10 scripts/vvtv-soak.sh
```

Checklist de release: `docs/release-checklist.md`.

Go-live consolidado da Fase 1:

```bash
scripts/vvtv-phase1-go-live.sh
```

O script gera artefatos em `runtime/go-live/phase1/<timestamp>/`.

Go-live consolidado da Fase 2 (runbook + backup/restore + soak curto por default):

```bash
scripts/vvtv-phase2-go-live.sh
```

O script gera artefatos em `runtime/go-live/phase2/<timestamp>/`.

Go-live consolidado da Fase 3 (canary + promote, com opcional de checagem Cloudflare):

```bash
scripts/vvtv-phase3-go-live.sh
```

O script gera artefatos em `runtime/go-live/phase3/<timestamp>/`.

## Canary release

Script de canary com rollback automatico:

```bash
VVTV_CANARY_SOAK_HOURS=1 scripts/vvtv-canary.sh
```

Canary com promote automatico (gate embutido):

```bash
VVTV_CANARY_AUTO_PROMOTE=1 VVTV_CANARY_SOAK_HOURS=1 scripts/vvtv-canary.sh
```

Dry run curto:

```bash
VVTV_CANARY_SOAK_HOURS=0 VVTV_CANARY_SOAK_MAX_SAMPLES=3 VVTV_CANARY_SOAK_INTERVAL_SECS=10 scripts/vvtv-canary.sh
```

## Promotion gate

Promove apenas com canary `PASS` e sem alertas `high/critical`:

```bash
scripts/vvtv-promote.sh runtime/canary/<timestamp>/result.env
```

## Recovery (SQLite)

- Estado persistido em `runtime/state/vvtv.db`
- Entidades persistidas: `PlanItem`, `AssetItem`, `QueueEntry`, `AuditEvent`
- No boot, o orquestrador tenta recovery da fila persistida antes de rodar um ciclo novo.

## Estrutura

- `apps/vvtv-orchestrator`: executa ciclo completo e recovery
- `apps/vvtv-control-api`: API `/v1` para status, reports e controle
- `apps/vvtv-admin`: CLI operacional (backup/restore de metadados)
- `crates/*`: modulos separados por responsabilidade
- `config/owner_card.sample.yaml`: politica inicial do canal
- `docs/runbook.md`: operacao e incidentes
- `cloudflare/worker`: stub de Worker + D1 para deploy futuro
