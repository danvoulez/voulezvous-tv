# Sprint 3 Runbook

## Prerequisites

- Control API rodando em `VVTV_API_URL` (default `http://127.0.0.1:7070`)
- `VVTV_CONTROL_TOKEN` e `VVTV_CONTROL_SECRET` alinhados com a API

## Comandos operacionais

Forcar nightly + retention/export:

```bash
scripts/vvtv-runbook.sh force-nightly
```

Forcar nightly e imprimir ultimo export de auditoria:

```bash
scripts/vvtv-runbook.sh export-audits
```

Rotacionar chave local de backup:

```bash
scripts/vvtv-runbook.sh backup-key-rotate
```

Backup de metadados (state + OwnerCard):

```bash
scripts/vvtv-runbook.sh backup-metadata
```

Backup de metadados criptografado:

```bash
scripts/vvtv-runbook.sh backup-metadata-secure
```

Restore de metadados:

```bash
scripts/vvtv-runbook.sh restore-metadata runtime/backups/<timestamp>
```

Restore de metadados criptografado:

```bash
scripts/vvtv-runbook.sh restore-metadata-secure runtime/backups/<timestamp>
```

Verificar backup sem restaurar:

```bash
scripts/vvtv-runbook.sh verify-backup runtime/backups/<timestamp>
```

Verificar backup criptografado:

```bash
scripts/vvtv-runbook.sh verify-backup-secure runtime/backups/<timestamp>
```

Ativar emergencia:

```bash
scripts/vvtv-runbook.sh emergency-on
```

Desativar emergencia:

```bash
scripts/vvtv-runbook.sh emergency-off
```

Toggle manual de emergencia:

```bash
scripts/vvtv-runbook.sh emergency-toggle
```

Canary com rollback automatico:

```bash
VVTV_CANARY_SOAK_HOURS=1 scripts/vvtv-canary.sh
```
