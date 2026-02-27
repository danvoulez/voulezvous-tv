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

Backup de metadados (state + OwnerCard):

```bash
scripts/vvtv-runbook.sh backup-metadata
```

Restore de metadados:

```bash
scripts/vvtv-runbook.sh restore-metadata runtime/backups/<timestamp>
```

Verificar backup sem restaurar:

```bash
scripts/vvtv-runbook.sh verify-backup runtime/backups/<timestamp>
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
