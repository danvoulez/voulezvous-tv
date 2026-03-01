# Plataforma Voulezvous — Documento Completo de Execução (Go-Live)

Este documento consolida e operacionaliza a visão de plataforma do ecossistema Voulezvous: **VVTV Broadcast 24/7**, **Control Plane multi-mode**, **abstração de mídia** e **Web Shell único por manifest**. O objetivo é sair de uma visão arquitetural para um plano de implantação completo, com critérios claros de aceite, operação e evolução.

---

## 1) Escopo e objetivo de negócio

A Plataforma Voulezvous deve operar em dois níveis integrados:

1. **Broadcast contínuo (voulezvous.tv)**: usuário anônimo assiste conteúdo sem login, com alta disponibilidade e fallback automático.
2. **Experiência interativa pós-login (Party e demais modes)**: presença, chat, convite para call privada, random/matching e futuros modos sem duplicar app ou backend.

Resultado esperado:

- Um único produto com múltiplos modos por composição.
- Reuso real de contratos, SDK, políticas e componentes.
- Evolução de novos modos por **configuração + manifest**, não por fork.

---

## 2) Estado atual da base (diagnóstico)

### 2.1 O que já está forte

A base atual cobre de forma sólida o núcleo local de operação VVTV:

- Pipeline modular em Rust (`discovery -> planner -> fetcher -> prep -> queue -> stream -> curate -> nightly`).
- Orquestração operacional com API de controle, auditoria e storage SQLite.
- Scripts de operação (runbook, canary, soak, promoção por fases).
- Integração Cloudflare Worker para camada leve de sincronização/status.

### 2.2 O que falta para “plataforma completa”

Os principais gaps para sair de “broadcast robusto” e chegar à “plataforma multi-mode completa” são:

- Control Plane cloud com **Session Orchestrator** canônico.
- Serviço de **Presence** com heartbeat/TTL e limpeza consistente.
- **Chat/Consent** e **Matching** integrados ao ciclo de sessão.
- Frontend com **Shell único** e composição por manifests.
- Contratos versionados e geração obrigatória de SDK/types.

---

## 3) Arquitetura-alvo (simples de operar, difícil de quebrar)

## 3.1 Macrocamadas

1. **LAB Plane (local, Rust)**
   - Descoberta, planejamento, fetch, QA, fila e streaming 24/7.
   - É o motor de continuidade de transmissão.

2. **Control Plane (cloud, event-driven)**
   - Identity/Auth, Presence, Session, Chat, Match, Moderation, Telemetry.
   - É o cérebro de experiência interativa multi-mode.

3. **Media Plane (provider-agnostic)**
   - WebRTC/HLS/ingest-egress via interface única.
   - É o plano de execução audiovisual desacoplado de fornecedor.

4. **Web App Shell (único frontend)**
   - `/` broadcast viewer e `/party` pós-login.
   - Outros modos por manifest + políticas.

### 3.2 Princípios mandatórios

- **Schemas primeiro**: API/eventos definidos em contrato versionado.
- **SDK gerado**: frontend e serviços não “copiam type na mão”.
- **Sessão é unidade de intenção**: mudança de modo cria nova sessão.
- **Policy centralizada**: regras de permissão e segurança sem duplicação.
- **Composição por capability**: modo declara capacidades, não reimplementa stack.

---

## 4) Modelo operacional de modos (manifest-driven)

Cada modo é um manifesto versionado contendo:

- `mode_id`
- `routes`
- `required_capabilities`
- `ui_composition`
- `policies`
- `analytics_events`

### 4.1 Modo Broadcast (viewer anônimo)

- Rota padrão `/`.
- MainPlayer (HLS) + topbar mínima.
- CTA de login/entrada.
- Sem painel administrativo para o usuário final.

### 4.2 Modo Party (padrão pós-login)

- Login redireciona diretamente para `/party`.
- Stage principal + rail de presença + overlay de chat.
- Convite para call privada passa por consentimento explícito.

### 4.3 Modo Random

- Entrada em fila, pareamento, sessão efêmera.
- Skip encerra sessão e reinicia fluxo com cooldown.
- Guardrails de abuso por política e reputação.

---

## 5) Contratos e domínio mínimo (v1)

## 5.1 Entidades

- **User**: identidade, reputação, flags de segurança.
- **Presence**: estado online, modo, visibilidade, heartbeat.
- **Session**: modo, estado (`creating|active|ending|ended|failed`), audience, parent_session.
- **ChatThread**: escopo (`private_1to1|room|performer_room`) + moderação.
- **MatchTicket**: fila, preferências, cooldown/quarentena.

## 5.2 Contratos obrigatórios v1

- `POST /sessions`
- `POST /sessions/transition`
- `POST /sessions/{id}/end`
- `GET /sessions/{id}`
- `POST /presence/set`
- `GET /presence/party`
- `POST /chat/thread`
- `WS /chat/{thread_id}`
- `POST /match/join`
- `POST /match/skip`

Todos os payloads devem nascer de schema versionado com validação runtime e geração de types.

---

## 6) Sequência de implementação (plano completo)

## Fase 0 — Hardening Broadcast (curta, imediata)

**Meta:** garantir base 24/7 como plataforma de confiança.

Entregas:

- Soak/canary estáveis.
- Runbook validado para incidentes reais.
- Alerting com dedupe e clear funcionando.

## Fase A — Fundação de contratos e estrutura

**Meta:** eliminar retrabalho estrutural.

Entregas:

- Estrutura dedicada:
  - `packages/schemas`
  - `packages/sdk`
  - `packages/config`
  - `services/control-plane`
  - `apps/web`
- Schemas v1 de session/presence/chat/match/consent.
- Geração de SDK/types e gate de CI para divergência.

## Fase B — Session Orchestrator + MediaProvider

**Meta:** núcleo de orquestração multi-mode.

Entregas:

- State machine de sessão com invariantes fortes.
- `transition` criando nova sessão com `parent_session_id`.
- Interface `MediaProvider` + provider inicial (staging).
- Perfis de mídia por uso (thumb, private call, random, broadcast).

## Fase C — Presence + Chat + Consent

**Meta:** experiência Party funcional ponta a ponta.

Entregas:

- Heartbeat/TTL/cleanup de presença.
- Chat thread único por escopo.
- Consent chain obrigatória para call privada.

## Fase D — Matching/Random com proteção

**Meta:** random utilizável sem explosão de custo e abuso.

Entregas:

- Queue/match/skip/cooldown configuráveis.
- Quarentena de conta nova e rate limits por tier.
- Telemetria de custo por sessão e por tentativa.

## Fase E — Web Shell manifest-driven

**Meta:** experiência unificada de produto.

Entregas:

- Viewer `/` + login -> `/party`.
- Componentes reutilizáveis (player, chat, rail, overlays).
- Modes por manifest sem telas duplicadas.

## Fase F — Operação de Go-Live controlado

**Meta:** abrir com segurança e rollback rápido.

Entregas:

- Rollout em etapas (internal -> beta -> público parcial -> público total).
- Gates automáticos de saúde por fase.
- Plano de rollback e pós-incidente padronizados.

---

## 7) Critérios de aceite (Go/No-Go)

A plataforma só passa para soft opening quando todos os critérios abaixo estiverem verdes:

1. **Broadcast 24/7 estável** com buffer saudável e fallback testado.
2. **Invariantes de sessão** sem órfãos e sem transição inválida.
3. **Consentimento obrigatório** sem bypass em backend.
4. **Guardrails de random** ativos (cooldown, rate-limit, quarentena).
5. **Shell único** em produção com manifests para modos.
6. **Telemetria mínima** por modo disponível em dashboard operacional.
7. **Runbook de incidente** testado em simulação de falha.

---

## 8) SLOs, métricas e observabilidade

### 8.1 SLOs iniciais sugeridos

- Uptime de reprodução broadcast: **>= 99.9%**.
- Tempo de recuperação de sessão (falha não fatal): **< 10s**.
- Match-to-connect no random (p50): **< 5s**.
- Erro de transição de sessão: **< 0.5%**.

### 8.2 KPIs de produto

- Conversão viewer -> login.
- Conversão login -> primeira interação (chat/call/random).
- Taxa de sucesso de consentimento.
- Retenção em Party e reincidência de abuso.

### 8.3 Telemetria mandatória por modo

- `mode_entered`
- `mode_exited`
- `session_created`
- `session_transitioned`
- `media_error`
- `policy_denied`

---

## 9) Segurança, compliance e confiança

- Consentimento explícito para chamadas privadas.
- Políticas centralizadas para report, block, ban e quarentena.
- Audit trail completo de decisões automáticas e ações humanas.
- Segredos e tokens segregados por ambiente.
- Bloqueio de credenciais de desenvolvimento em produção.

---

## 10) Riscos críticos e mitigação

1. **Duplicação de app por modo**
   - Mitigação: shell único + manifest + lint de fronteiras.

2. **Lock-in de provedor de mídia**
   - Mitigação: `MediaProvider` único e testes por adapter.

3. **Custos invisíveis em presence/random**
   - Mitigação: caps por tier, viewport gating, budget de reconnect.

4. **Quebra de confiança (calls sem consent)**
   - Mitigação: validação server-side obrigatória + testes de bypass.

5. **Incidentes sem rastreabilidade**
   - Mitigação: auditoria estruturada + export de evidências por incidente.

---

## 11) Plano de rollout (recomendado)

### Etapa 1 — Internal alpha

- Time interno e contas de teste.
- Foco em estabilidade e falhas de fluxo crítico.

### Etapa 2 — Beta fechado

- Grupo pequeno de usuários reais.
- Observação de custo, QoS e comportamento de moderação.

### Etapa 3 — Público parcial

- Tráfego fracionado com feature flags.
- Rollback automático se violar SLO.

### Etapa 4 — Público amplo

- Escala gradual com monitoramento de KPIs de retenção.
- Revisões semanais de policy + ajustes finos.

---

## 12) Backlog objetivo (30 dias)

### Semana 1

- Estrutura de schemas/sdk/control-plane/web shell criada.
- Contratos v1 publicados e validados.
- Gates de CI para schema e geração de SDK.

### Semana 2

- Session orchestrator funcional em staging.
- MediaProvider com provider inicial e testes básicos.
- Presença heartbeat/TTL ativa.

### Semana 3

- Chat + consent + convite para private call completos.
- Random com join/match/skip/cooldown.
- Telemetria mínima end-to-end.

### Semana 4

- Shell web com `/` + `/party` + manifest de random.
- Hardening operacional (runbook e rollback ensaiados).
- Início de beta fechado.

---

## 13) Definição de pronto (DoD) por modo

Um modo só é considerado pronto quando:

- Não cria endpoint fora de schema.
- Não duplica type nem validação fora da fonte de verdade.
- Reusa componentes de UI kit.
- Usa policy central e telemetria padrão.
- Passa testes de fluxo feliz + anti-bypass + falha controlada.

---

## 14) Próximo passo prático (ação imediata)

Sequência recomendada para início sem retrabalho:

1. Criar branch de execução de plataforma.
2. Subir Fase A (schemas + geração + gates de CI).
3. Implementar Session Orchestrator (Fase B).
4. Acoplar Presence/Chat/Consent (Fase C).
5. Entregar shell `/` e `/party` em staging (Fase E).

Com essa ordem, a plataforma evolui de forma incremental, auditável e com risco controlado, sem sacrificar o funcionamento contínuo do broadcast atual.
