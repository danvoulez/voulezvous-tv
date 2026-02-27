# VVTV Runbook

## SLO
- Disponibilidade alvo: 99.5%
- Buffer alvo: 60 min
- Threshold critico: 20 min

## Alertas principais
- `buffer_minutes < 20`
- `qa_pass_rate` abaixo de baseline
- `fallback_rate` em crescimento continuo
- falha recorrente por dominio allowlist

## Acoes de emergencia
1. Ativar `POST /v1/control/emergency-mode`.
2. Recarregar politica (`reload-owner-card`) se houve mudanca de regra.
3. Verificar eventos em `AuditEvent` com `reason_code` de falha.

## Backup/restore
- Rotacionar chave local de criptografia:
  - `scripts/vvtv-runbook.sh backup-key-rotate`
- Garantir politica de idade da chave (rotaciona se vencida):
  - `scripts/vvtv-runbook.sh backup-key-ensure`
- Limpar chaves antigas de arquivo:
  - `scripts/vvtv-runbook.sh backup-key-prune`
- Gerar snapshot completo (`state.db` + `OwnerCard` + `manifest` com SHA-256):
  - `scripts/vvtv-runbook.sh backup-metadata`
- Gerar snapshot criptografado em repouso:
  - `scripts/vvtv-runbook.sh backup-metadata-secure`
- Restaurar snapshot:
  - `scripts/vvtv-runbook.sh restore-metadata runtime/backups/<timestamp>`
- Restaurar snapshot criptografado:
  - `scripts/vvtv-runbook.sh restore-metadata-secure runtime/backups/<timestamp>`
- Verificar integridade antes de restore:
  - `scripts/vvtv-runbook.sh verify-backup runtime/backups/<timestamp>`
- Verificar integridade de snapshot criptografado:
  - `scripts/vvtv-runbook.sh verify-backup-secure runtime/backups/<timestamp>`
- Defaults:
  - `VVTV_STATE_DB_PATH=runtime/state/vvtv.db`
  - `VVTV_OWNER_CARD_PATH=config/owner_card.sample.yaml`
  - `VVTV_BACKUP_DIR=runtime/backups`
  - `VVTV_BACKUP_KEY_DIR=runtime/keys`

## Canary e rollback
- Rodar canary com backup automatico e rollback em falha:
  - `VVTV_CANARY_SOAK_HOURS=1 scripts/vvtv-canary.sh`
- Auto-promote opcional no mesmo fluxo:
  - `VVTV_CANARY_AUTO_PROMOTE=1 scripts/vvtv-canary.sh`
- Promover apenas se canary PASS e sem alertas high/critical:
  - `scripts/vvtv-promote.sh runtime/canary/<timestamp>/result.env`
- Artefatos:
  - `runtime/canary/<timestamp>/result.env`
  - `runtime/canary/<timestamp>/soak/summary.txt`
