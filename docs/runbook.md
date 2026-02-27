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
- Gerar snapshot completo (`state.db` + `OwnerCard` + `manifest` com SHA-256):
  - `scripts/vvtv-runbook.sh backup-metadata`
- Restaurar snapshot:
  - `scripts/vvtv-runbook.sh restore-metadata runtime/backups/<timestamp>`
- Defaults:
  - `VVTV_STATE_DB_PATH=runtime/state/vvtv.db`
  - `VVTV_OWNER_CARD_PATH=config/owner_card.sample.yaml`
  - `VVTV_BACKUP_DIR=runtime/backups`
