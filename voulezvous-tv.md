# voulezvous.tv

**Uma TV autônoma que roda 24/7 sem precisar de você.**

Ela descobre conteúdo, decide o que baixar, verifica qualidade e transmite em HLS sem parar — tudo guiado por um arquivo de regras que você define uma vez.

---

## O problema que ela resolve

A maioria dos sistemas de autopilot quebra de três formas previsíveis: baixa tudo até o disco explodir, não tem fallback quando a web muda de baixo dos seus pés, e quando algo dá errado ninguém sabe quem decidiu o quê.

VVTV resolve isso com um pipeline de checkpoints onde cada etapa tem responsabilidade clara e audit trail completo.

---

## Como funciona

O pipeline vai do mais leve ao mais comprometido:

```
Discover → Plan → Commit (T-4h) → Fetch → Prep/QA → Queue → Stream → Curate → Nightly
```

**Discover** — navega e valida fontes reais de playback, não só URLs.

**Plan** — monta intenções leves sem baixar nada. Só planos.

**Commit T-4h** — 4 horas antes do ar decide o que vira real e baixa só isso.

**Prep/QA** — normaliza áudio/vídeo, checa integridade. Se reprovar, entra a reserva automaticamente.

**Queue/Stream** — mantém buffer de 60 minutos e publica HLS contínuo. Modo emergência garante que nunca cai.

**Curate** — ajusta repetição e diversidade dentro dos limites que você definiu.

**Nightly** — ajuste fino diário, máximo 3% de variação, sem mexer no estilo do dono.

---

## Arquitetura

Tudo que importa roda local em Rust. Cloudflare entra só como camada leve de controle e relatórios.

### LAB (Mac Mini / Rust)

| Crate | Responsabilidade |
|---|---|
| `vvtv-discovery` | Navegação real e validação de playback/HD |
| `vvtv-planner` | Geração de planos + reservas + grade preliminar |
| `vvtv-fetcher` | Resolução final T-4h + fallback + download |
| `vvtv-prep` | Remux/normalização + QA técnico |
| `vvtv-queue` | Fila de execução + buffer + modo emergência |
| `vvtv-stream` | Empacotamento HLS |
| `vvtv-curator` | Reordenação/substituição conforme política |
| `vvtv-nightly` | Ajuste fino diário dentro dos guardrails |
| `vvtv-audit` | Trilha de eventos e decisões |
| `vvtv-store` | Persistência SQLite (estado + reports + alertas) |

### Cloudflare (leve)

- Worker API para comandos mínimos e status
- D1 para metadados resumidos e relatórios
- R2 opcional para snapshots

---

## OwnerCard — a fonte única de política

O OwnerCard é a alma do sistema. Você define uma vez, e a VVTV obedece.

```yaml
schema_version: 1
editorial_profile:
  style: "documentary"
  rhythm: "balanced"

search_policy:
  allowlist_domains: ["youtube.com", "vimeo.com"]
  blacklist_domains: ["exemplo-ruim.com"]

schedule_policy:
  plan_horizon_hours: 24
  commit_interval_minutes: 30

quality_policy:
  min_width: 1280
  min_height: 720
  audio_target_lufs: -14

curator_policy:
  mode: "auto"
  max_reorders_per_hour: 6

safety_policy:
  forbidden_terms: []
  watermark_blacklist: []

autotune_policy:
  nightly_time: "03:00"
  max_daily_delta_pct: 3
```

Se você subir um OwnerCard inválido, o sistema mantém o anterior. Sem surpresas.

---

## Quickstart

```bash
# 1. Clone e rode os testes
git clone https://github.com/danvoulez/voulezvous-tv
cd voulezvous-tv
cargo test -q

# 2. Rode o orquestrador (modo one-shot)
VVTV_RUN_ONCE=1 cargo run -p vvtv-orchestrator

# 3. Rode a Control API
cargo run -p vvtv-control-api
# /v1/status | /v1/reports/daily | /v1/reports/weekly
```

**Dependências:** Rust stable, `ffmpeg` + `ffprobe` no PATH, SQLite (embutido).

---

## Operação

Scripts assinados com HMAC para comandos críticos:

```bash
# Emergência
scripts/vvtv-runbook.sh emergency-on
scripts/vvtv-runbook.sh emergency-off

# Rotinas
scripts/vvtv-runbook.sh force-nightly
scripts/vvtv-runbook.sh export-audits

# Backups
scripts/vvtv-runbook.sh backup-metadata
scripts/vvtv-runbook.sh verify-backup runtime/backups/<timestamp>
scripts/vvtv-runbook.sh restore-metadata runtime/backups/<timestamp>
```

---

## Alertas

Dispatcher a cada 60s via webhook, só envia `high` e `critical`, com dedupe por código e cooldown de 900s. Envia `alert_clear` quando normaliza.

```bash
VVTV_ALERT_WEBHOOK_URL=https://seu-endpoint.com
VVTV_ALERT_COOLDOWN_SECS=900  # default
```

---

## Segurança

Em produção o sistema bloqueia tokens de desenvolvimento por padrão e exige configuração explícita:

```bash
VVTV_ENV=prod
VVTV_CONTROL_TOKEN=<token>
VVTV_CONTROL_SECRET=<secret>
VVTV_CLOUDFLARE_BASE_URL=<url>  # se usar controle remoto
```

---

## O que fica registrado

Tudo. Cada decisão tem rastro:

- Planos criados (intenções)
- Commits T-4h (o que virou real)
- Resultados de QA (pass/fail + motivo)
- O que foi ao ar e quando
- Cada ação do curador (antes/depois + score)
- Alertas (ativo/clear + estado de dedupe)
- Ajustes nightly (delta aplicado)

Você audita o sistema como caixa preta. Sem burocracia.

---

## Roadmap

- Criptografia de backups (age/gpg)
- Rotação automática de snapshots
- Canary/rollback automatizado com soak gates
- Hardening de discovery (anti-bot, DOM drift, health por domínio)
- Release gates automáticos (buffer + QA + backup_verified)

---

## Nota sobre conteúdo

VVTV é infraestrutura técnica. O que entra no ar depende do seu OwnerCard — suas fontes, seus critérios, suas regras.

---

## Licença

All rights reserved. © LogLine Foundation