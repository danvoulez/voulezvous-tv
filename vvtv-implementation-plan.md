# VVTV / Voulezvous — Plano de Implementação Sequenciado por Risco

**Versão:** 1.0  
**Data:** 2026-03-01  
**Contexto:** Solo founder, Cloudflare-first, LAB local (Mac Minis), soft opening Janeiro 2026  
**Premissa:** Cada fase entrega valor testável. Nada avança sem as invariantes da fase anterior estarem cobertas por testes.

---

## Mapa de Fases

```
Fase 0 ─── Fundação (config + contracts + skeleton)
  │
Fase 1 ─── Session Orchestrator + MediaProvider (o coração)
  │
Fase 2 ─── Presence + Thumbs WebRTC (o maior multiplicador de custo)
  │
Fase 3 ─── Chat + Consent (a cadeia de confiança)
  │
Fase 4 ─── Matching + Random (o mais exposto a abuso)
  │
Fase 5 ─── Broadcast / LAB Pipeline (o motor 24/7)
  │
Fase 6 ─── Shell + UI Composition (frontend unificado)
  │
Fase 7 ─── Moderação + Safety reforçados
  │
Fase 8 ─── Telemetria + Observabilidade
  │
Fase 9 ─── Polish + Soft Opening
```

---

## Fase 0 — Fundação (config + contracts + skeleton)

**Risco que mitiga:** Tudo que vem depois herda decisões desta fase. Erro aqui = retrabalho em cascata.

**Por que primeiro:** Os 3 YAMLs (media-profiles, mode-manifests, cost-tiers) já existem como spec. Transformá-los em fonte de verdade executável antes de escrever qualquer serviço garante que todo serviço nasce já consumindo config, não hardcode.

### Entregas

1. **Monorepo scaffold**
   - `/packages/schemas` — JSON Schemas versionados (OpenAPI + Event Schemas)
   - `/packages/core` — utilitários: errors, tracing stubs, rate-limit primitives, feature flags
   - `/packages/domain` — entidades puras (User, Presence, Session, ChatThread, MatchTicket) sem IO
   - `/packages/config` — loader + deepMerge + validateConfig + buildRuntimeConfig
   - `/services/` — stubs vazios com interfaces definidas
   - `/lab/` — stub do pipeline

2. **Config pipeline funcional**
   - `media-profiles.yaml` carregável e validável
   - `mode-manifests.yaml` carregável e validável
   - `cost-tiers.yaml` com deep-merge aplicável
   - `validateConfig()` passando em CI (todos os checks do doc: audio off em thumbs, tile limits, cooldown schedule, separação de ecossistemas, referências cruzadas)
   - `buildRuntimeConfig()` produzindo ConfigBundle tipado

3. **Contracts (schemas)**
   - Schemas de API para Session (create/transition/end/get)
   - Schemas de eventos WebSocket (presence.update, session.created, match.found, chat.message, consent.request/response)
   - Geração de types TS a partir dos schemas (CI falha se schema mudar sem regenerar)

4. **CI pipeline mínimo**
   - Lint de boundaries (domain não importa services, schemas não depende de nada)
   - Validação de config (os checks)
   - Golden tests de payloads contra schemas

### Critério de done
- `npm run validate:config` passa com os 3 YAMLs
- `npm run generate:types` produz types consumíveis
- CI verde com boundary lint + schema validation
- Zero types duplicados entre packages

### Estimativa: 1-2 semanas

---

## Fase 1 — Session Orchestrator + MediaProvider

**Risco que mitiga:** É o coração. Todo modo depende dele. Race conditions aqui = sessões orfãs, vazamento de recursos de mídia, custo fantasma.

**Por que agora:** Sem Session Orchestrator funcional, nada mais funciona. E o MediaProvider adapter precisa existir antes de qualquer WebRTC, senão o código de mídia se espalha e depois não desacopla.

### Entregas

1. **Session Orchestrator (Cloudflare Worker + Durable Object)**
   - `POST /sessions` — cria sessão com: mode, audience_type, stream_transport, intent_label, policy snapshot
   - `POST /sessions/transition` — encerra sessão atual, cria nova, retorna handover hint (cut/crossfade)
   - `POST /sessions/{id}/end` — encerra com cleanup
   - `GET /sessions/{id}` — estado atual
   - State machine: `creating → active → ending → ended | failed`
   - Lineage: `parent_session_id` sempre preenchido em transições
   - **Invariante crítica:** transição SEMPRE cria nova sessão. Nunca reinterpreta sessão existente.
   - Timeout automático: sessão sem heartbeat → `ending → ended` com revoke de endpoints

2. **MediaProvider interface + adapter Cloudflare Realtime**
   - Interface: `createWebRTCEndpoints(session_id, profile_name)`, `revokeEndpoints(session_id)`, `createTurnCredentials(session_id)`
   - Adapter Cloudflare: implementa a interface chamando Realtime SFU/TURN APIs
   - Perfis de mídia: recebe `profile_name` (ex: `thumb_low`, `call_random_fast`), resolve caps do config
   - **Invariante crítica:** Session Orchestrator nunca sabe qual provider está por trás. Só chama a interface.

3. **Testes de invariantes**
   - Test: criar sessão → transição → sessão original está `ended`
   - Test: transição sem encerrar anterior → falha
   - Test: sessão sem heartbeat por N segundos → auto-end + revoke endpoints
   - Test: duas transições simultâneas na mesma sessão → apenas uma vence (lock)
   - Test: revoke falha no provider → sessão marca `failed` + alerta
   - Test: perfil de mídia inexistente → erro antes de criar endpoint

### Critério de done
- Session Orchestrator deployed em Cloudflare (staging)
- MediaProvider adapter criando endpoints reais no Realtime
- Suite de testes de invariantes passando
- Logs de custo: cada sessão registra profile usado + duração estimada

### Estimativa: 2-3 semanas

---

## Fase 2 — Presence + Thumbs WebRTC

**Risco que mitiga:** Maior multiplicador de custo. Cada tile é egress contínuo. Viewport gating quebrado = custo explode silenciosamente.

**Por que agora:** Presence é a base de Party e Sniffers. E é onde o custo precisa estar sob controle antes de abrir pra usuários.

### Entregas

1. **Presence Service (Durable Object + WebSocket Hibernation)**
   - WebSocket `/presence` com heartbeat (TTL curto, anti-fantasma)
   - Campos: presence_id, user_id, presence_mode, is_camera_on, last_seen_at, visibility, capabilities
   - Endpoints: `GET /presence/party`, `GET /presence/grid`, `POST /presence/set`
   - **Invariante:** presença é UMA API com filtros. Party vs grid = query + policy, não serviço separado.
   - Hibernation: WS idle → hibernated (custo de DO cai drasticamente)
   - Anti-fantasma: sem heartbeat por 15s → presença removida da lista

2. **Thumb WebRTC integration**
   - Tile conecta via MediaProvider usando perfil `thumb_low`
   - **Viewport gating:** só tiles visíveis recebem stream. Tile sai da viewport → disconnect imediato.
   - **Hard caps enforced:** audio OFF sempre, 180p, 8fps, 180kbps max (do config)
   - Auto-degrade: rede ruim → 120p/4fps (sem reconexão, só downgrade de layer)
   - Budget: max `tiles_active` por device (12 desktop, 6 mobile) — enforced no client E no backend

3. **Cost monitoring primitivo**
   - Log por viewer: `tiles_connected × bitrate_estimado × tempo`
   - Alerta se egress estimado ultrapassar threshold do kill_switch

4. **Testes de invariantes**
   - Test: 13º tile no desktop → recusa conexão (limit 12)
   - Test: tile fora de viewport por 2s → WebRTC disconnected
   - Test: heartbeat para → presença some em < 20s
   - Test: thumb profile → audio sempre OFF
   - Test: rede simulada ruim → downgrade automático, não reconexão
   - Test: DO em hibernation → WS reconecta limpo quando volta

### Critério de done
- Presence funcionando em staging com WebSocket real
- Thumbs WebRTC renderizando com viewport gating
- Cost estimation logging em cada viewer session
- Testes de invariantes passando

### Estimativa: 2-3 semanas

---

## Fase 3 — Chat + Consent Handshake

**Risco que mitiga:** Consent que falha = destruição de confiança. Chat é a ponte entre presença e calls.

**Por que agora:** Chat é prerequisito para invite-to-call. Consent é prerequisito para Private Call. E Private Call é o flow mais comum do Party.

### Entregas

1. **Chat Service (Durable Object por thread)**
   - `POST /chat/thread` — cria ou reabre thread
   - `WS /chat/{thread_id}` — mensagens em tempo real
   - `POST /chat/{thread_id}/report` — report com metadata
   - Scopes: `private_1to1 | room | performer_room` — mesmo modelo, policy diferente
   - Rate limiting por scope (live público = mais restritivo)
   - **DRY:** Thread model único. Escopo muda por policy, não por implementação.

2. **Consent Handshake**
   - Invite: usuário A envia convite via chat ou ProfileCard
   - Delivery: push via WS para usuário B → `ConsentOverlay`
   - Aceite: backend cria Session `private_call` via Session Orchestrator
   - Recusa: feedback para A, B volta ao contexto anterior
   - Timeout: convite expira em N segundos (config: `timeout_s: 20`)
   - **Invariante crítica:** NENHUMA sessão de call é criada sem consent explícito. Zero exceção.

3. **Testes de invariantes**
   - Test: call session criada sem consent → impossível (API recusa)
   - Test: consent timeout → convite cancelado, nenhuma sessão criada
   - Test: dois consents simultâneos para mesmo usuário → queue ou recusa do segundo
   - Test: consent aceito → Session created com `intent_label: private_call`
   - Test: consent recusado → nenhum side effect, ambos voltam ao estado anterior
   - Test: thread já existe entre A e B → reabre, não duplica
   - Test: chat rate limit em live público → mensagem bloqueada com feedback

### Critério de done
- Chat funcional com threads 1-1
- Consent handshake end-to-end (invite → overlay → accept/reject → session)
- Zero cenário onde call começa sem consent
- Rate limiting testado

### Estimativa: 2 semanas

---

## Fase 4 — Matching + Random

**Risco que mitiga:** Mais exposto a abuso. Skip spam, bot floods, denial-of-wallet. Se os guardrails não funcionarem, o custo explode antes de você ver.

**Por que agora:** Random depende de Session Orchestrator (Fase 1) e MediaProvider (Fase 1). É o modo mais caro por minuto e precisa dos controles de custo validados antes do soft opening.

### Entregas

1. **Matching Service (Worker + DO)**
   - `POST /match/join` — entra na fila com prefs
   - `POST /match/skip` — skip com cooldown enforcement
   - `WS /match` — notificação de match found
   - **Separação de responsabilidades:** Matching só decide match. Session Orchestrator cria a sessão. MediaProvider aloca endpoints.
   - Queue por segmento (prefs/language/tags)

2. **Anti-abuso (enforcement desde o dia 1)**
   - **Cooldown progressivo:** schedule do config `[0, 0, 0, 3, 5, 8, 15, 30, 60]`
   - **Min time before skip:** 2000ms (config) — skip antes disso = recusado
   - **Reconnect budget:** max 2 attempts/minute com backoff
   - **Quarantine:** conta nova (< 3 dias) = quality reduzida + frequência limitada
   - **Max matches/hour:** 20 para quarantine, 60 para COST_LOW normal
   - **Max skips/minute:** 12 (kill_switch)
   - Todos esses valores vêm do config (cost-tiers.yaml), não são hardcoded

3. **Cadeia completa testável**
   - join → searching → match found → session created → WebRTC connected → skip → session ended → cooldown → searching
   - Cada transição gera evento de telemetria

4. **Testes de invariantes**
   - Test: skip antes de `min_time_before_skip_ms` → recusado
   - Test: 4º skip rápido → cooldown de 3s aplicado
   - Test: 60+ matches/hora → bloqueio
   - Test: conta nova → perfil forçado para `call_random_fast` com caps reduzidos
   - Test: reconnect 3x em 1 minuto → recusado (budget: 2)
   - Test: cooldown_schedule é non-decreasing após índice 3 → CI valida
   - Test: match found → Session Orchestrator chamado (não matching diretamente)
   - Test: skip encerra sessão atual E reentra na fila (não navega)

### Critério de done
- Random end-to-end funcionando (join → match → call → skip → rematch)
- Todos os guardrails anti-abuso ativos e testados
- Cost estimation por sessão Random
- Quarantine funcional para contas novas

### Estimativa: 2-3 semanas

---

## Fase 5 — Broadcast / LAB Pipeline

**Risco que mitiga:** Se o LAB falha, voulezvous.tv fica sem conteúdo. Mas é risco operacional, não arquitetural — a TV pode rodar com playlist estática enquanto o pipeline evolui.

**Por que agora:** O broadcast é a porta de entrada (/) e o "fundo permanente" do Party. Precisa existir antes do frontend.

### Entregas

1. **Pipeline local (Mac Mini)**
   - **Planner:** gera planos (intenções) aplicando OwnerCard + regras de variedade
   - **Fetcher:** download T-4h, confirma disponibilidade, fallbacks
   - **Prepare & QA:** container fix, normalização de áudio, checks de integridade, reprova/troca
   - **Broadcast Queue:** fila com ~1h de folga, sempre tem próximo pronto
   - **Transmitter:** publica HLS (LAB-first, sem Cloudflare Stream)

2. **Modo emergência**
   - Fallback: loop de reservas se pipeline falhar
   - Health check contínuo (se queue < 30min → alerta)
   - Restart automático do transmitter

3. **Lab Agent (ponte LAB ↔ nuvem)**
   - Recebe comandos: start/stop, reload policies, rotate playlist
   - Envia: status, telemetria resumida, health
   - API de controle simples (REST autenticado)

4. **OwnerCard / Policy Service**
   - Armazena/versão políticas editoriais e operacionais
   - Policy snapshot usado pelo Planner e pelo Session Orchestrator
   - Audit trail de mudanças

### Critério de done
- HLS stream funcionando a partir do LAB
- Modo emergência testado (kill pipeline → fallback ativa em < 30s)
- Lab Agent reportando status para a nuvem
- OwnerCard versionado e auditável

### Estimativa: 2-3 semanas

---

## Fase 6 — Shell + UI Composition

**Risco que mitiga:** Se o frontend não for composição, cada modo vira app separado e o DRY morre.

**Por que agora:** Com todos os backends (Sessions, Presence, Chat, Matching, Broadcast) funcionando, o frontend pode compor sobre serviços reais. Construir UI antes dos backends = mocks que divergem.

### Entregas

1. **Shell único**
   - 5 regiões: Stage, Rail/Drawer, Overlays, TopBar, ModeRouter
   - ModeRouter como state machine: lê mode-manifests.yaml, monta composição
   - TransitionLayer (crossfade/cut/placeholder)

2. **UI Kit (componentes base)**
   - `MainPlayer` (HLS), `AudioPlayer`, `VideoTile`, `PresenceRail`, `GridDiscovery`
   - `ChatOverlay`, `MatchOverlay`, `ConsentOverlay`, `ModerationSheet`
   - `CallUI`, `MeetingGrid`, `RoomChat`, `MirrorView`
   - `ProfileCard` (com ações: chat, invite, report, block)
   - **Regra:** componentes sem `if (mode === X)`. Composição via manifest + props.

3. **SDK client (gerado)**
   - Types + client gerados a partir dos schemas
   - Frontend consome SDK, nunca chama API diretamente

4. **Modos funcionais (composição)**
   - `/` — Broadcast Viewer (MainPlayer + login CTA)
   - `/party` — Party (MainPlayer + PresenceRail + ChatOverlay)
   - `/sniffers` — Grid (GridDiscovery + FiltersPanel + ChatOverlay)
   - `/random` — Random (CallUI + MatchOverlay + controls)
   - `/call/:token` — Invite (ConsentOverlay → CallUI)
   - `/mirror` — Mirror (MirrorView + "Abrir para…")

5. **Microcopy centralizada**
   - `copy.json` como fonte de verdade para todas as strings
   - Nenhuma string hardcoded em componente

### Critério de done
- Shell renderizando todos os modos com dados reais
- Navegação sem dashboard: login → Party, deep links funcionando
- Componentes reutilizados entre modos (zero duplicação)
- Mobile: drawer + bottom sheet (não nova tela)

### Estimativa: 3-4 semanas

---

## Fase 7 — Moderação + Safety Reforçados

**Risco que mitiga:** Sem moderação robusta, abuse escala rápido (especialmente Random e Live Public).

**Por que agora:** Os modos já funcionam. Agora precisa proteger os usuários que vão usá-los.

### Entregas

1. **Moderation Service completo**
   - Reports com metadata (session_id, thread_id, mode_id, reason_code)
   - Block: bidirecional, imediato, persiste entre sessões
   - Ban: temporal ou permanente, por admin/mod
   - Shadow/quarantine: usuário limitado sem saber (para investigação)
   - Consent handshake reforçado (já existe da Fase 3, agora com edge cases)

2. **Separação de ecossistemas (enforcement real)**
   - Party presence NÃO vaza para Live Public
   - Random queue é isolada do Party
   - `live_public.policies.separate_ecosystem_from_party === true` enforced em todas as queries
   - `live_public.rail.component !== presence_rail` validado em CI + runtime

3. **UI de moderação (1 clique)**
   - Report + Block acessível em todo contexto (ProfileCard, Chat, Random, Live)
   - Feedback imediato (toast)
   - Quarantine UI para contas novas (funcionalidade limitada)

4. **Identity & Auth completo**
   - Tokens, sessão, device binding opcional
   - Flags: idade/consent, bans, reputation
   - Middleware único reutilizável (backend) + guard no frontend via SDK
   - RBAC leve: admin, mod, user

### Critério de done
- Report → block → ban flow funcional end-to-end
- Separação de ecossistemas testada e enforced
- Contas novas em quarantine automaticamente
- Auth com flags e reputation funcionando

### Estimativa: 2 semanas

---

## Fase 8 — Telemetria + Observabilidade

**Risco que mitiga:** Sem telemetria, você não sabe se o custo está explodindo, se os guardrails estão funcionando, ou se os usuários estão tendo problemas.

**Por que agora:** Todos os modos estão funcionando. Antes de abrir, precisa de olhos.

### Entregas

1. **Telemetry ingestion**
   - Eventos do `telemetry-events.yaml` implementados
   - Funil: app_open → view_broadcast → click_login → auth_success → enter_mode → ...
   - QoS sampling (10% default, 100% em incidentes)
   - Privacy: nunca loga message bodies, video frames, geolocation precisa

2. **Cost dashboard**
   - Egress estimado por modo por hora
   - Fórmula real: `GB/h = 0.439 × Mbps_recebidos × viewers_concorrentes`
   - Alerta se ultrapassar `max_estimated_egress_gb_per_hour_total` do kill_switch
   - Random: `call_minutes × bitrate × 2 (bidirecional)`

3. **Status / Monitoring**
   - Health de cada serviço (Presence, Sessions, Matching, Broadcast)
   - Alertas: broadcast down, presence travou, matching degradou, egress spike
   - Lab status (queue health, emergency mode active)

4. **Audit Log Service**
   - Decisões do Curator (o que foi ao ar, quando, por quê)
   - Decisões de moderação (ban, quarantine, report resolution)
   - Versionamento de policies (OwnerCard changes)

### Critério de done
- Dashboard de custo funcional (mesmo que simples)
- Funil de telemetria capturando eventos de todos os modos
- Alertas de health configurados
- Audit log registrando decisões

### Estimativa: 2 semanas

---

## Fase 9 — Polish + Soft Opening

**Risco que mitiga:** Problemas de UX e edge cases que só aparecem com uso real.

### Entregas

1. **Stress test com guardrails**
   - Simular N viewers com presence thumbs → verificar cost estimation
   - Simular skip spam no Random → verificar cooldown + quarantine
   - Simular broadcast pipeline failure → verificar emergency mode

2. **Mobile polish**
   - Drawer/bottom sheet behavior
   - Touch targets
   - Responsive transitions

3. **Skins / Clone infra**
   - Tema padrão Voulezvous
   - Demonstrar que "clone" = tema + layout + routes + policies (sem fork)

4. **Discovery Service (read-only sobre Presence)**
   - Grid/sniffers: ranking, filtros, segmentos
   - Basicamente queries sobre Presence + perfil + policy

5. **Edge cases e hardening**
   - Reconexão após perda de rede
   - Token expirado durante call
   - Usuário fecha tab durante consent handshake
   - Dois dispositivos do mesmo usuário

### Critério de done
- Stress test passa sem surpresa de custo
- Mobile funcional e responsivo
- Edge cases documentados e testados
- Pronto para beta fechado

### Estimativa: 2-3 semanas

---

## Timeline Consolidada

| Fase | Escopo | Duração | Acumulado |
|------|--------|---------|-----------|
| 0 | Fundação (config + contracts) | 1-2 sem | 2 sem |
| 1 | Session Orchestrator + MediaProvider | 2-3 sem | 5 sem |
| 2 | Presence + Thumbs WebRTC | 2-3 sem | 8 sem |
| 3 | Chat + Consent | 2 sem | 10 sem |
| 4 | Matching + Random | 2-3 sem | 13 sem |
| 5 | Broadcast / LAB Pipeline | 2-3 sem | 16 sem |
| 6 | Shell + UI Composition | 3-4 sem | 20 sem |
| 7 | Moderação + Safety | 2 sem | 22 sem |
| 8 | Telemetria + Observabilidade | 2 sem | 24 sem |
| 9 | Polish + Soft Opening | 2-3 sem | 27 sem |

**Total estimado: ~27 semanas (~6.5 meses)**

Considerando que você está solo mas com AI-assist pesado, as fases podem se comprimir. O paralelismo natural é: Fase 5 (LAB) pode rodar em paralelo com Fases 3-4 (são independentes). Isso pode comprimir para ~22-24 semanas.

---

## Mapa de Dependências

```
Fase 0 (fundação)
├── Fase 1 (sessions + media) ─── depende de: config, schemas
│   ├── Fase 2 (presence) ─── depende de: sessions, media provider
│   │   └── Fase 6 (UI) ─── depende de: presence, chat, matching, broadcast
│   ├── Fase 3 (chat + consent) ─── depende de: sessions
│   │   └── Fase 6 (UI)
│   └── Fase 4 (matching + random) ─── depende de: sessions, media provider
│       └── Fase 6 (UI)
├── Fase 5 (broadcast / LAB) ─── depende de: config, policy service
│   └── Fase 6 (UI)
├── Fase 7 (moderação) ─── depende de: auth, sessions, chat
├── Fase 8 (telemetria) ─── depende de: todos os serviços
└── Fase 9 (polish) ─── depende de: UI + todos os serviços
```

**Caminho crítico:** 0 → 1 → 2 → 6 → 9  
**Paralelizáveis:** Fase 5 com Fases 2-4 | Fase 7 com Fase 6 | Fase 8 com Fase 9

---

## Invariantes Globais (valem pra TODAS as fases)

1. **Transição de modo = nova Session.** Nunca reinterpretar sessão existente.
2. **Nenhuma call sem consent explícito.** Zero exceção.
3. **Thumbs nunca com áudio.** Enforced em config, CI, e runtime.
4. **Viewport gating é lei.** Tile fora da view = disconnect.
5. **Separação de ecossistemas é enforcement, não combinado.** CI + runtime.
6. **Config é fonte de verdade.** Nenhum cap hardcoded em serviço ou componente.
7. **Clamps só reduzem, nunca aumentam.** Kill switches são ceiling, não floor.
8. **Cada modo é composição, não app.** Se precisou de `if (mode === X)`, algo está errado.
9. **Schema first.** Endpoint novo sem schema = CI falha.
10. **Custo é observável.** Cada sessão sabe quanto está custando (estimativa).
