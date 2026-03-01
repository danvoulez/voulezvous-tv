# Plataforma Voulezvous Completa — Plano de Colocar em Pé (Prático)

Este documento traduz os objetivos de **`voulezvous-tv.md`**, do **Engineering Doc modular multi-mode** e do **`vvtv-implementation-plan.md`** para uma execução pragmática em cima da codebase atual.

## 1) Onde estamos hoje (estado real do repositório)

A base atual já cobre bem o núcleo de **VVTV Broadcast/LAB**:

- Pipeline modular em Rust (`discovery -> planner -> fetcher -> prep -> queue -> stream -> curate -> nightly`).
- Orquestrador contínuo + Control API + Store SQLite + auditoria/alertas.
- Integração Cloudflare Worker (stub operacional para sync/status).
- Scripts operacionais: runbook, soak, canary e promoção.

Em termos de plataforma completa (Party, Random, Private Call, Meeting, Presence, Session Orchestrator cloud), ainda faltam blocos de **Control Plane multi-mode** e **UI shell por manifests**.

## 2) Definição objetiva de “Plataforma completa em pé”

Para considerar a plataforma "em pé" com risco controlado, precisamos de 4 trilhas simultâneas:

1. **Broadcast robusto (já avançado)**
   - VVTV 24/7 estável (buffer, fallback, QA, runbook, canary).
2. **Control Plane multi-mode mínimo viável**
   - Session Orchestrator + Presence + Chat/Consent + Matching.
3. **Media abstraction única**
   - Interface `MediaProvider` por perfil (evitar hardcode por modo).
4. **Frontend shell único por manifest**
   - `/` viewer broadcast e login levando direto para Party.

## 3) Ordem de implementação recomendada (anti-retrabalho)

A ordem abaixo respeita o plano por risco (`vvtv-implementation-plan.md`) e reduz custo de reescrita:

### Fase A — Fundação de contratos/config (curta e mandatória)

**Objetivo:** parar de tomar decisão estrutural no código e mover para schemas/config.

Entregáveis:

- Criar árvore de plataforma (sem quebrar o Rust atual):
  - `packages/schemas`
  - `packages/config`
  - `packages/sdk`
  - `apps/web` (shell)
  - `services/control-plane`
- Definir schemas v1 para:
  - Session (`create/transition/end/get`)
  - Presence update
  - Chat message + report
  - Match found
  - Consent request/response
- Gerar types/SDK a partir de schema (CI deve falhar se divergir).

### Fase B — Session Orchestrator + MediaProvider

**Objetivo:** coração do sistema multi-mode.

Entregáveis:

- API de sessões com state machine (`creating -> active -> ending -> ended|failed`).
- Transição sempre criando nova sessão (com `parent_session_id`).
- `MediaProvider` interface desacoplada do provider concreto.
- Primeira implementação provider: Cloudflare Realtime (ou adapter mock + staging).

### Fase C — Presence + Chat/Consent

**Objetivo:** login em Party com interação real e confiável.

Entregáveis:

- Presence com heartbeat/TTL (sem usuários fantasmas).
- `ChatThread` único por escopo (`private_1to1`, `room`, `performer_room`).
- Consent obrigatório para abrir `private_call`.

### Fase D — Matching/Random com guardrails

**Objetivo:** abrir Random sem explodir custo e sem abuso.

Entregáveis:

- Fila/match/skip/cooldown configuráveis.
- Quarentena de conta nova.
- Rate limits e reconnect budget em config.

### Fase E — Shell Web (viewer -> login -> party)

**Objetivo:** experiência produto com um app só.

Entregáveis:

- `/` = broadcast viewer sem login.
- login -> redirect direto para `/party`.
- UI por composition/manifests (sem clonar app por modo).

## 4) Mapeamento direto com a codebase atual (o que reaproveitar)

- **Reaproveitar 100%** do pipeline Rust atual como LAB/Broadcast core.
- **Manter** `vvtv-control-api` como base de controle operacional do LAB.
- **Adicionar** control-plane multi-mode como novo bloco, sem destruir módulos existentes.
- **Usar** `vvtv-store` e `vvtv-audit` como origem de eventos para telemetria e auditoria cruzada.

## 5) Backlog executável (primeiros 14 dias)

### Semana 1

1. Criar `packages/schemas` com contrato mínimo (session/presence/chat/match/consent).
2. Pipeline de geração de types SDK (TS) e check de CI.
3. Stub de `services/control-plane` com rotas vazias, mas tipadas por schema.
4. Documento de `Mode Manifest v1` (Party, Random, Broadcast).

### Semana 2

1. Implementar Session Orchestrator em staging.
2. Implementar adapter `MediaProvider` com profile resolution.
3. Testes de invariantes de transição/heartbeat/cleanup.
4. Publicar primeiro `apps/web` shell com `/` + `/party` (sem features complexas).

## 6) Critérios de aceite (go/no-go de plataforma)

A plataforma só entra em "soft opening" quando TODOS abaixo estiverem verdes:

1. **Broadcast 24/7** com soak pass e canary pass.
2. **Session invariants** passando (sem sessão órfã; transição cria nova sessão).
3. **Consent chain** sem bypass possível.
4. **Random guardrails** ativos e validados (cooldown/quarentena/rate limits).
5. **Shell único** operando com manifests sem duplicação de app.

## 7) Riscos críticos (e mitigação)

- **Risco:** duplicar frontend/backend por modo.
  - **Mitigação:** modo só nasce via manifest + capability.
- **Risco:** provider lock-in e lógica de mídia espalhada.
  - **Mitigação:** tudo atrás de `MediaProvider`.
- **Risco:** custo silencioso em Presence/Random.
  - **Mitigação:** viewport gating, caps por tier, telemetria de custo por sessão.
- **Risco:** confiança quebrada por call sem consent.
  - **Mitigação:** invariante de backend + testes bloqueando bypass.

## 8) Próximo passo imediato (ação concreta)

Ordem para começar amanhã sem travar:

1. Abrir branch de trabalho "platform-phase-a".
2. Subir `packages/schemas` + geração de types.
3. Conectar CI para falhar em divergência de schema/types.
4. Só então iniciar Session Orchestrator.

Se quiser, no próximo passo eu preparo o **pacote Fase A pronto para commit** (estrutura de pastas + schemas v1 iniciais + scripts de geração + validação em CI) para você já entrar em execução.
