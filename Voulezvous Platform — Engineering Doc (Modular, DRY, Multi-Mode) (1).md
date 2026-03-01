  
VVTV / Voulezvous Platform — Engineering Doc (Modular, DRY, Multi-Mode)  
\=======================================================================

0\) Objetivo  
\-----------

Construir \*\*um único produto/plataforma\*\* com múltiplos “modes” (Party, Sniffers, Random, Live Public, Private Call, Meeting, Mirror etc.), onde:

\*   \*\*voulezvous.tv\*\* é o modo “Broadcast” (assistir sem login).  
\*   \*\*login → entra no Party Mode\*\* (sem dashboard).  
\*   cada modo é \*\*composição\*\* de capacidades existentes (Presence \+ Session \+ Chat \+ Discovery \+ Moderation \+ Media), \*\*sem reimplementar lógica\*\*.

O sistema deve permitir criar novos clones \*\*mudando configuração \+ UI composition\*\*, não copiando backend/frontend.

\* \* \*

1\) Princípios de Arquitetura (anti-código repetido)  
\---------------------------------------------------

1\.  \*\*Um Core, muitas skins\*\*    
    “Clone” \= tema \+ layout \+ rotas \+ regras de permissão \+ composição de componentes.    
    Nada de “fork de app”.  
2\.  \*\*Contratos primeiro (schemas como fonte da verdade)\*\*    
    Tudo (API, eventos, payloads) nasce em \*\*schemas versionados\*\*.    
    SDKs e types são \*\*gerados\*\*, não escritos duas vezes.  
3\.  \*\*Capabilities, não features hard-coded\*\*    
    Um modo declara “o que usa” (presence, chat, matching, broadcast overlay…) e o shell monta.  
4\.  \*\*Transição de modo \= nova Session\*\*    
    Nunca reinterpretar o mesmo vídeo/estado em outro simbolismo.  
5\.  \*\*Separação de intenções por policy\*\*    
    Party presence ≠ Performer stage ≠ Random queue.    
    Essa separação é garantida por \*\*políticas e roteamento\*\*, não por “combinado”.

\* \* \*

2\) Visão de Alto Nível (camadas)  
\--------------------------------

\#\#\# 2.1 Control Plane (estado e real-time)

\*   Identity & Auth  
\*   Presence  
\*   Session Orchestrator (criação/transição/encerramento)  
\*   Chat  
\*   Discovery  
\*   Matching  
\*   Moderation & Safety  
\*   Analytics/Telemetry  
\*   OwnerCard (Cartão do Dono)

\> Control plane é \*\*event-driven\*\* e precisa de consistência por “sala/sessão”.

\#\#\# 2.2 Media Plane (vídeo/áudio)

\*   WebRTC (calls, thumbnails, meetings)  
\*   HLS (broadcast / main content)  
\*   SFU/TURN (infra de conectividade)  
\*   Ingest/Egress (quando houver live public)

\> Media plane fica atrás de uma \*\*MediaProvider interface\*\* (plugável), pra não espalhar lógica de mídia.

\#\#\# 2.3 LAB Plane (VVTV “fábrica” local)

\*   Navegação/validação de fontes  
\*   Planos, download T-4h, preparação e QA  
\*   Fila e transmissão 24/7  
\*   Curadoria local

\> O LAB conversa com a nuvem via \*\*API de controle\*\* e publica métricas/resumos.

\* \* \*

3\) Monorepo (para DRY real)  
\---------------------------

\#\#\# 3.1 Estrutura sugerida

\*   \`/packages/schemas\`    
    JSON Schemas/OpenAPI \+ Event Schemas (fonte da verdade)  
\*   \`/packages/sdk\`    
    client SDK gerado (TS/Swift/etc.)  
\*   \`/packages/core\`    
    utilitários compartilhados (auth, errors, tracing, rate-limit, feature flags)  
\*   \`/packages/domain\`    
    entidades, invariantes, policies (sem IO)  
\*   \`/packages/ui-kit\`    
    componentes base (VideoTile, ChatOverlay, PresenceRail, ModeRouter…)  
\*   \`/apps/web\`    
    shell único (carrega “modes” via manifest)  
\*   \`/services/\*\`    
    serviços do control plane (ou módulos)  
\*   \`/lab/\*\`    
    pipeline VVTV local (planner/downloader/transmitter)

\#\#\# 3.2 Regra anti-duplicação

\*   Nada de “copiar types” pra frontend/backend: \*\*tudo vem de \`/schemas\`\*\*.  
\*   Nada de “reimplementar client”: usar \*\*SDK gerado\*\*.  
\*   Nada de “repetir lógica de permissão”: policies em \`/domain\` e aplicadas por middleware no backend \+ guard no frontend.

\* \* \*

4\) Contratos (single source of truth)  
\-------------------------------------

\#\#\# 4.1 Schemas (API \+ Eventos)

\*   \*\*OpenAPI\*\* para REST (ou HTTP endpoints)  
\*   \*\*Event schemas\*\* para WebSocket messages e pub/sub interno

Gatilho de build:

\*   \`schemas\` → gera:  
    \*   types TS  
    \*   validators runtime  
    \*   SDK client  
    \*   docs (human-readable)

\#\#\# 4.2 Versionamento

\*   \`v1\` como baseline  
\*   mudanças breaking só com \`v2\` (e período de dupla escrita/leitura quando necessário)

\* \* \*

5\) Modelo de Domínio (núcleo)  
\-----------------------------

\#\#\# 5.1 Entidades Core

\*\*User\*\*

\*   id, flags (idade/consent), reputation, bans, tiers

\*\*Presence\*\*

\*   presence\\\_id, user\\\_id  
\*   presence\\\_mode: \`party\_presence | grid\_presence | random\_waiting | performer\_live\`  
\*   is\\\_camera\\\_on, last\\\_seen\\\_at, capabilities  
\*   visibility: \`party\_only | grid\_only | hidden\`

\*\*Session\*\*

\*   session\\\_id, mode: \`mirror | party | random | live\_public | private\_call | meeting | sniffers | broadcast\`  
\*   state: \`creating | active | ending | ended | failed\`  
\*   audience\\\_type: \`self | public | matched | invited | room\`  
\*   audience\\\_ids (quando aplicável)  
\*   stream\\\_transport: \`webrtc | hls | rtmp\`  
\*   parent\\\_session\\\_id (transições)  
\*   intent\\\_label (evita mistura simbólica)  
\*   policy snapshot (flags aplicadas no momento da criação)

\*\*ChatThread\*\*

\*   thread\\\_id, scope: \`private\_1to1 | room | performer\_room\`  
\*   participants, created\\\_from, moderation metadata

\*\*MatchTicket\*\*

\*   ticket\\\_id, user\\\_id, queue, prefs  
\*   cooldown\\\_until, quarantine flags

\#\#\# 5.2 VVTV (Broadcast)

\*\*OwnerCard\*\*

\*   regras editoriais/operacionais (versionado, auditável)

\*\*Plan\*\*

\*   intenção leve (não é asset), score de estilo, janela recomendada

\*\*Asset\*\*

\*   arquivo/stream pronto, QA results, fingerprint, provenance/licensing metadata

\*\*BroadcastQueueItem\*\*

\*   o que foi ao ar, quando, por quê

\*\*AuditLog\*\*

\*   decisões e evidências (essencial para auditoria)

\* \* \*

6\) Serviços/Módulos (com interfaces claras)  
\-------------------------------------------

\#\#\# 6.1 Identity & Auth

\*   tokens, sessão, device binding opcional  
\*   middleware único (reutilizável)  
\*   guard no frontend via SDK

\#\#\# 6.2 Presence Service

\*   WebSocket \`/presence\`  
\*   TTL/heartbeat (anti-fantasma)  
\*   endpoints:  
    \*   \`GET /presence/party\`  
    \*   \`GET /presence/grid\`  
    \*   \`POST /presence/set\`

\*\*Regra DRY:\*\* presença é uma API única com filtros; “party vs grid” é query/policy, não outro serviço.

\#\#\# 6.3 Session Orchestrator

\*   \`POST /sessions\`  
\*   \`POST /sessions/transition\`  
\*   \`POST /sessions/{id}/end\`  
\*   \`GET /sessions/{id}\`

\*\*Transição sempre cria nova sessão\*\* e retorna “handover hint” (cut/crossfade).

\#\#\# 6.4 Chat Service

\*   \`POST /chat/thread\`  
\*   \`WS /chat/{thread\_id}\`  
\*   \`POST /chat/{thread\_id}/report\`

\*\*DRY:\*\* Thread model único; “private vs room vs performer” muda \`scope\` \+ policy.

\#\#\# 6.5 Discovery Service

\*   grid de presenças (thumbs)  
\*   filtros e ranking

\*\*DRY:\*\* discovery é leitura sobre Presence \+ policies. Nada de duplicar “lista de online”.

\#\#\# 6.6 Matching Service (Random)

\*   \`POST /match/join\`  
\*   \`POST /match/skip\`  
\*   \`WS /match\`

\*\*DRY:\*\* matchmaking só entrega “match found”; Session Orchestrator cria a sessão.

\#\#\# 6.7 Moderation & Safety

\*   reports, bans, quarantine/shadow, consent handshake  
\*   aplica policies via middleware comum

\*\*DRY:\*\* política não fica “espalhada”; fica central e é consumida por todos.

\#\#\# 6.8 Analytics/Telemetry

\*   eventos de funil \+ QoS \+ auditoria  
\*   schema versionado (mesma fonte)

\* \* \*

7\) Media Provider (plugável) — pra não espalhar código de vídeo  
\---------------------------------------------------------------

\#\#\# 7.1 Interface única

Uma camada \`MediaProvider\` responde a operações:

\*   \`createWebRTCEndpoints(session\_id, roles, constraints)\`  
\*   \`createBroadcastOutput(queue\_item)\`  
\*   \`createIngest(room\_id)\` (para performer)  
\*   \`createEgress(session\_id)\` (se aplicável)  
\*   \`revokeEndpoints(session\_id)\`

O Session Orchestrator \*\*não sabe\*\* se é Cloudflare/LiveKit/mediasoup: só chama a interface.

\#\#\# 7.2 Perfis de mídia (reuso por modo)

Defina perfis prontos:

\*   \`presence\_thumb\_no\_audio\_low\_bitrate\`  
\*   \`private\_call\_standard\`  
\*   \`meeting\_grid\`  
\*   \`random\_fast\_connect\`  
\*   \`live\_public\_stage\`  
\*   \`broadcast\_hls\`

Cada modo referencia um perfil. Sem duplicar tuning.

\* \* \*

8\) “Modes” como manifests (DRY no frontend \+ regras no backend)  
\---------------------------------------------------------------

\#\#\# 8.1 Mode Manifest (config)

Cada modo é um arquivo de config (versionado) com:

\*   \`mode\_id\`  
\*   \`routes\`  
\*   \`required\_capabilities\` (presence/chat/matching/broadcast…)  
\*   \`ui\_composition\` (componentes \+ layout)  
\*   \`policies\` (o que pode, o que não pode)  
\*   \`analytics\_events\` (nomes padronizados)

\#\#\# 8.2 Shell único

O app web é um \*\*shell\*\* com:

\*   \`ModeRouter\` (state machine)  
\*   \`Overlays\` (ChatOverlay, MatchOverlay)  
\*   \`Players\` (MainPlayer, PresenceTiles)  
\*   \`SDK client\` (gerado)

\*\*Sem clones de app.\*\* Só manifests \+ temas.

\* \* \*

9\) Fluxos obrigatórios  
\----------------------

\#\#\# 9.1 Viewer → Login → Party

\*   Usuário anônimo assiste broadcast  
\*   clica login  
\*   pós-login: redirect para \`/party\` (a menos que exista deep link explícito tipo convite)

\#\#\# 9.2 Party → Chat → Invite → Private Call

\*   clicar numa presença cria/abre thread 1–1  
\*   “invite to call” cria Session \`private\_call\` com consent handshake  
\*   transição suave com placeholder/crossfade

\#\#\# 9.3 Random (skip sem tropeço)

\*   join queue → match found → create session  
\*   skip encerra sessão atual e reentra na fila (cooldown/policy)

\* \* \*

10\) Frontend (UI Kit reutilizável)  
\----------------------------------

\#\#\# 10.1 Componentes base (um kit só)

\*   \`MainPlayer\` (broadcast)  
\*   \`AudioPlayer\` (music)  
\*   \`VideoTile\` (thumb webrtc/hls)  
\*   \`PresenceRail\`  
\*   \`GridDiscovery\`  
\*   \`ChatOverlay\`  
\*   \`MatchOverlay\`  
\*   \`CallUI\`  
\*   \`RoomChat\`  
\*   \`ModeRouter\`

\#\#\# 10.2 Regras anti-duplicação

\*   Componentes sem “if (mode===X)” gigante.    
    Em vez disso: composition via manifest \+ props “capabilities”.

\* \* \*

11\) Garantias de Qualidade (gates de build)  
\-------------------------------------------

Para impedir regressão e duplicação:

1\.  \*\*Lint de boundaries\*\*  
    \*   \`/domain\` não importa \`/services\` nem \`/apps\`  
\*   \`/schemas\` não depende de nada  
\*   \`/ui-kit\` não importa código de modo específico  
    2\.  \*\*SDK obrigatório\*\*  
    \*   endpoints novos exigem schema \+ geração do SDK (CI falha se não)  
    3\.  \*\*Golden tests de eventos\*\*  
    \*   payloads validados contra schema (frontend e backend)  
    4\.  \*\*Telemetry mínimo por modo\*\*  
    \*   eventos de entrada/saída de modo e erros de mídia

\* \* \*

12\) Como adicionar um novo “clone” sem repetir código (passo a passo)  
\---------------------------------------------------------------------

1\.  Criar \`mode\_manifest\` novo (routes \+ composição \+ policies).  
2\.  Reutilizar componentes do UI Kit (sem criar tela do zero).  
3\.  Se precisar de comportamento novo:  
    \*   primeiro tenta como \*\*capability\*\* (ex.: “presence \+ overlay X”)  
    \*   se virar padrão, sobe pro UI Kit / Core  
4\.  Se precisar de backend novo:  
    \*   primeiro tenta como \*\*policy \+ query\*\* em serviço existente  
    \*   só cria módulo novo se for realmente outra entidade/fluxo  
5\.  Atualiza schemas → gera SDK → pronto.

\* \* \*

13\) “Definition of Done” (DRY de verdade)  
\-----------------------------------------

Um modo está “pronto” quando:

\*   não existe endpoint “exclusivo” sem schema  
\*   não existe type duplicado  
\*   não existe UI duplicada (reusa kit)  
\*   policy está centralizada  
\*   media usa \`MediaProvider\` e perfis  
\*   telemetry mínima existe

\* \* \*

Se você quiser, eu transformo isso no próximo passo em \*\*duas coisas bem práticas\*\*:

1\.  Um \*\*Mode Manifest exemplo\*\* (Party / Sniffers / Random) com composição e policies (sem tecnicês, mas implementável).  
2\.  Um \*\*mapa de eventos\*\* (presence/session/chat/match) com nomes padronizados pra você ter fluidez e auditoria sem inventar payload toda hora.

Quer que eu faça os exemplos em \*\*JSON\*\* (manifest \+ schemas) ou em \*\*TypeScript types\*\* (mais confortável pra dev)?

\#\# Prompt:  
Quero a descrição da UI

\#\# Response:  
Thought for 8s

Fechou, Dan 😄 Vou te passar uma \*\*descrição de UI\*\* bem “engenharia de produto”: \*\*shell único \+ componentes reutilizáveis \+ composição por modo\*\*, com estados, transições e microinterações — tudo pensado pra \*\*não duplicar telas nem lógica\*\*.

\* \* \*

UI Spec — Voulezvous (VVTV \+ Party \+ Modes)  
\===========================================

1\) Estrutura geral: \*\*um app, vários modos\*\*  
\--------------------------------------------

A UI é um \*\*Shell\*\* único que carrega “modes” via configuração (manifest), sem apps separados.

\#\#\# 1.1 O Shell (sempre presente)

\*   \*\*Stage (conteúdo principal)\*\*: onde fica o vídeo principal (VVTV ou uma Call).  
\*   \*\*Overlays\*\* (camadas por cima, independentes do Stage):  
    \*   \`ChatOverlay\` (privado 1–1 / room)  
    \*   \`MatchOverlay\` (random)  
    \*   \`ConsentOverlay\` (aceitar/recusar chamada)  
    \*   \`ModerationSheet\` (report/block)  
    \*   \`Toasts\` (status rápidos)  
\*   \*\*Rail / Drawer\*\* (coluna lateral ou gaveta):  
    \*   \`PresenceRail\` (Party)  
    \*   \`QueuePanel\` (Random)  
    \*   \`RoomInfoPanel\` (Live Public/Meeting)  
\*   \*\*TopBar minimalista\*\* (sem “dashboard vibe”):  
    \*   logo/voltar, indicador de modo, botão de login/perfil, status de rede

\> Ideia-chave: o \*\*ModeRouter\*\* só troca \_configuração \+ composição\_, não troca app.

\* \* \*

2\) Porta de entrada: \*\*voulezvous.tv (deslogado)\*\*  
\--------------------------------------------------

\#\#\# 2.1 Tela “Broadcast Viewer” (default)

\*\*Layout\*\*

\*   Centro: \`MainPlayer\` (HLS)  
\*   Canto superior: TopBar leve (logo \+ “Entrar”)  
\*   Rodapé discreto (opcional): “O que está passando agora” \+ relógio \+ próxima troca (se você quiser)

\*\*Interação\*\*

\*   Usuário só assiste.  
\*   CTA principal: \*\*Entrar\*\*.

\#\#\# 2.2 Regra de ouro do login (UX)

\*   Clicou \*\*Entrar\*\* → autentica → \*\*cai direto no Party Mode\*\*.  
\*   Sem dashboard, sem home intermediária.  
\*   Transição visual: \*\*crossfade\*\* ou “continua tocando” e abre o Party como camada.

\*\*Estados\*\*

\*   Loading: placeholder suave (sem tela branca)  
\*   Falha de login: toast \+ volta ao player do jeito que estava

\* \* \*

3\) Party Mode (modo “mundo vivo”)  
\---------------------------------

Party é a “casa” do usuário logado.

\#\#\# 3.1 Layout Party (desktop)

\*   \*\*Stage\*\*: \`MainPlayer\` (canal principal — broadcast)  
\*   \*\*Camada de áudio\*\*: \`AudioPlayer\` (música do Party, se existir)  
\*   \*\*Lateral direita\*\*: \`PresenceRail\`  
    \*   mini tiles (webcam thumbs) com status “online agora”  
    \*   \*\*sem áudio\*\* nos thumbs por padrão  
\*   \*\*Overlay flutuante\*\*: \`ChatOverlay\` (abre por cima sem tirar o Stage)

\#\#\# 3.2 Layout Party (mobile)

\*   Stage fullscreen  
\*   \`PresenceRail\` vira \*\*drawer\*\* (puxa da lateral/baixo)  
\*   Chat abre como \*\*bottom sheet\*\* (não navega pra outra tela)

\#\#\# 3.3 Microinterações (Party)

\*   Tap/click em um \`VideoTile\` de presença:  
    \*   abre \*\*ProfileCard\*\* mini (nome/flags/tags/online)  
    \*   ações: \*\*Chat\*\* (texto), \*\*Convidar para call\*\*  
\*   Chat privado:  
    \*   abre \`ChatOverlay\` (não sai do Party)  
    \*   tem ações rápidas: “Report”, “Block”, “Invite to call”  
\*   Invite to call:  
    \*   dispara \`ConsentOverlay\` pro outro lado  
    \*   se aceito → abre \`CallUI\` (nova Session) como \*\*transição suave\*\*  
    \*   Party pode continuar como “ambiente” (ex.: áudio atenuado) ou pausar, por policy

\#\#\# 3.4 Indicadores essenciais

\*   Badge de conexão (verde/amarelo/vermelho)  
\*   “Online agora” confiável (sem fantasmas)  
\*   “Camera on/off” por tile  
\*   Botão rápido “Desligar minha cam” (privacy)

\* \* \*

4\) Sniffers / Grid (descoberta visual)  
\--------------------------------------

\#\#\# 4.1 Layout

\*   \*\*Stage\*\*: \`GridDiscovery\` (mosaico grande)  
\*   TopBar: filtros (tags, idioma, online, etc.)  
\*   \`ChatOverlay\` igual ao Party (reuso total)

\#\#\# 4.2 Interações

\*   Tap em tile → abre ProfileCard → Chat / Invite  
\*   Scroll infinito (com paginação)  
\*   Filtros persistem por sessão (não reinicia toda hora)

\*\*Importante:\*\* Sniffers usa o mesmo \*\*componente de tile\*\* e o mesmo \*\*ChatOverlay\*\* do Party.

\* \* \*

5\) Random (Chatroulette-like)  
\-----------------------------

\#\#\# 5.1 Layout

\*   \*\*Stage\*\*: \`CallUI\` (vídeo 1–1)  
\*   Overlay: \`MatchOverlay\` (estado da roleta)  
    \*   “Procurando…”  
    \*   “Conectando…”  
    \*   “Conectado”  
\*   Controles fixos:  
    \*   \*\*Skip\*\*  
    \*   Mute cam/mic  
    \*   Report/Block (sempre visível)

\#\#\# 5.2 Estados do Random (muito importante)

1\.  \*\*Idle\*\* (antes de entrar)  
2\.  \*\*Searching\*\* (fila)  
3\.  \*\*Found\*\* (match encontrado)  
4\.  \*\*Connecting\*\* (WebRTC)  
5\.  \*\*Active\*\* (chamada)  
6\.  \*\*Skipped / Ended\*\* (encerra e volta a Searching com cooldown)  
7\.  \*\*Cooldown\*\* (se abuso, conta nova, etc.)

\> “Skip” nunca navega pra outro lugar: só troca estado e cria nova Session.

\* \* \*

6\) Private Call (convite)  
\-------------------------

\#\#\# 6.1 Layout

\*   \*\*Stage\*\*: \`CallUI\`  
\*   Overlay obrigatório: \`ConsentOverlay\` no começo (aceitar/recusar)  
\*   \`ChatOverlay\` pode existir paralelo (texto durante call), por policy

\#\#\# 6.2 UX do convite

\*   Link/convite abre direto a tela de consentimento  
\*   Se recusado: volta ao Party (ou ao contexto anterior)  
\*   Se aceito: Call inicia, com transição limpa (sem reload)

\* \* \*

7\) Meeting (sala com N)  
\-----------------------

\#\#\# 7.1 Layout

\*   \*\*Stage\*\*: \`MeetingGrid\`  
    \*   tiles com speaking indicator (ou highlight do ativo)  
\*   Lateral/drawer: \`RoomInfoPanel\` (participantes, permissões)  
\*   Chat: \`ChatOverlay\` em modo room (reuso)

\#\#\# 7.2 Controles

\*   Mute mic/cam  
\*   “Levantar mão” (opcional)  
\*   Host controls (se existir): silenciar/remover, travar sala

\* \* \*

8\) Live Public (performer stage)  
\--------------------------------

\*\*Separação total do Party\*\* (intenção diferente).

\#\#\# 8.1 Layout

\*   \*\*Stage\*\*: \`PerformerStagePlayer\` (live)  
\*   Lateral: \`RoomChat\` (chat público)  
\*   TopBar: info da room, regras, report

\#\#\# 8.2 Interações

\*   Chat público com rate limit  
\*   Botões: seguir, report, (tips opcional no futuro)

\* \* \*

9\) Mirror (self view)  
\---------------------

\#\#\# 9.1 Layout

\*   \*\*Stage\*\*: \`MirrorView\` (câmera local)  
\*   Controles: cam/mic, espelho, ajustes rápidos  
\*   CTA: “Abrir para…” (gera nova Session, nunca “vira público do nada”)

\* \* \*

10\) Navegação: \*\*sem dashboard mental\*\*  
\---------------------------------------

\#\#\# 10.1 Regra de navegação

\*   “Voltar” sempre retorna ao \*\*ambiente anterior\*\* sem resetar tudo:  
    \*   se estava no Party → volta pro Party  
    \*   se veio por link de convite → volta pro Party após call  
\*   O app evita páginas “menu”: tudo é \*\*modo \+ overlays\*\*.

\#\#\# 10.2 Deep links

\*   \`/party\` (padrão pós-login)  
\*   \`/sniffers\`  
\*   \`/random\`  
\*   \`/call/invite/:token\`  
\*   \`/meeting/:room\_id\`  
\*   \`/live/:room\_id\`  
\*   \`/mirror\`

\* \* \*

11\) Moderação no UI (sempre a 1 clique)  
\---------------------------------------

Em qualquer lugar que tenha interação:

\*   \`Report\` \+ \`Block\` sempre acessível (sheet)  
\*   feedback imediato (“recebido”, “bloqueado”)  
\*   se necessário, “quarantine UI” (limita funções para conta nova)

\* \* \*

12\) Skins / Clones (sem duplicar UI)  
\------------------------------------

Um “clone” muda:

\*   tema (cores, tipografia, ícones)  
\*   copy/strings  
\*   ordem/posição das peças (layout)  
\*   quais modos aparecem (feature flags)  
\*   políticas (ex.: Party com/sem music)

Mas reaproveita:

\*   \`MainPlayer\`, \`VideoTile\`, \`PresenceRail\`, \`ChatOverlay\`, \`CallUI\`, \`ModeRouter\`, etc.

\* \* \*

Se você quiser, eu fecho isso com um “mapa de telas”  
\====================================================

Me diz só uma coisa rapidinha (pra eu não chutar errado): no Party, você quer \*\*o broadcast principal sempre tocando\*\* (VVTV como “fundo permanente”) \*\*ou\*\* em alguns modos ele pausa/saí (por exemplo, quando entra em call)?

Se você disser “sempre tocando”, eu te devolvo a versão final com:

\*   árvore de layout (desktop/mobile),  
\*   regras exatas de transição (crossfade vs pause),  
\*   e estados/empty/error por componente.

\#\# Prompt:  
Reler inteiro e refinar já com mapa de telas

\#\# Response:  
Thought for 11s

Fechado, Dan ✅ Reli tudo aqui e vou te devolver uma versão \*\*refinada\*\*, já \*\*com mapa de telas\*\*, mantendo o espírito do plano único: \*\*VVTV (broadcast) → login → Party → modos\*\*. Tudo \*\*modular/DRY\*\*, com \*\*um Shell\*\* e \*\*composições por modo\*\*, sem duplicar telas nem lógica.

\* \* \*

UI Doc (refinado) — Voulezvous.tv / VVTV \+ Party \+ Modes  
\========================================================

1\) Premissas do produto (pra UI não se perder)  
\----------------------------------------------

\*   \*\*É um plano só / um produto só.\*\* Não existem “apps” diferentes: existem \*\*modes\*\*.  
\*   \*\*A porta de entrada é o Broadcast\*\* (voulezvous.tv) — assistir sem login.  
\*   \*\*Login sempre leva ao Party Mode\*\* (sem dashboard).  
\*   \*\*Party \= estado presente\*\* (porn \+ música \+ “who’s online now” com cams flutuantes \+ chat privado paralelo).  
\*   \*\*Separação de intenções é lei\*\*:  
    \*   Party presence \*\*não\*\* vira “performer stage”.  
    \*   Live Public (rooms) \*\*tem discovery próprio\*\* e UI própria.  
    \*   Random é outro ecossistema (anti-abuso sempre visível).  
\*   \*\*Trocar de modo não pode tropeçar\*\*: transição suave, overlays, sem “reset mental”.

\* \* \*

2\) Arquitetura de UI: 1 Shell \+ Composição por Mode  
\===================================================

2.1 O Shell (estrutura fixa)  
\----------------------------

O Shell define 5 regiões. Os modos só “plugam” conteúdo nelas:

1\.  \*\*Stage (Principal)\*\*    
    Onde fica o “conteúdo dominante”:  
    \*   Broadcast Player (VVTV) \*\*ou\*\*  
    \*   Call UI (Random/Private/Meeting) \*\*ou\*\*  
    \*   Performer Stage (Live Public)  
2\.  \*\*Rail/Drawer (Lateral ou gaveta)\*\*  
    \*   PresenceRail (Party)  
    \*   Filtros/Lista (Sniffers)  
    \*   QueuePanel (Random)  
    \*   RoomInfo (Meeting/Live)  
3\.  \*\*Overlays (camadas por cima, reusáveis)\*\*  
    \*   ChatOverlay (privado 1–1 e room)  
    \*   ConsentOverlay (aceitar/recusar call)  
    \*   ModerationSheet (report/block)  
    \*   Toasts/Alerts (rede, status, erros)  
4\.  \*\*TopBar minimalista\*\* (sem “dashboard vibe”)  
    \*   logo \+ indicador de modo  
    \*   entrar/perfil  
    \*   status de conexão/câmera  
    \*   botão rápido “voltar ao Party” (quando fizer sentido)  
5\.  \*\*ModeRouter (máquina de estados)\*\*    
    Orquestra:  
    \*   qual modo está ativo  
    \*   o que fica no Stage  
    \*   quais overlays ficam disponíveis  
    \*   transições (cut/crossfade/placeholder)

\> DRY real: \*\*os componentes são os mesmos\*\*, muda a \*\*composição\*\* e algumas policies.

\* \* \*

3\) Mapa de telas (rotas) — versão canônica  
\==========================================

3.1 Rotas principais (1ª camada)  
\--------------------------------

\*   \`/\` → \*\*Broadcast Viewer\*\* (default público)  
\*   \`/party\` → \*\*Party Mode\*\* (default pós-login)  
\*   \`/sniffers\` → \*\*Grid Discovery\*\*  
\*   \`/random\` → \*\*Random 1–1\*\*  
\*   \`/call/:token\` → \*\*Invite → Consent → Private Call\*\*  
\*   \`/meeting/:room\_id\` → \*\*Meeting\*\*  
\*   \`/live/:room\_id\` → \*\*Live Public Room\*\*  
\*   \`/mirror\` → \*\*Mirror\*\*  
\*   \`/status\` → \*\*Status/health (opcional)\*\*

3.2 Deep links (regra)  
\----------------------

\*   Se o usuário abre \*\*um link de convite\*\* (\`/call/:token\`, \`/meeting/...\`) isso \*\*tem prioridade\*\*.  
\*   Sem deep link explícito → \*\*pós-login sempre /party\*\*.

\* \* \*

4\) Mapa de telas (flow) — do macro ao micro  
\===========================================

4.1 Flow A — Visitante (anônimo)  
\--------------------------------

1\.  Entra em \`/\`  
2\.  Assiste Broadcast (VVTV)  
3\.  Clica “Entrar”  
4\.  Autentica  
5\.  Redireciona para \`/party\` (\*\*regra fixa\*\*)  
6\.  Party abre como “mundo vivo” (presence \+ chat)

4.2 Flow B — Party → Chat → Invite → Call → volta  
\-------------------------------------------------

1\.  Em \`/party\`, toca em uma presença  
2\.  Abre ProfileCard → clica “Chat”  
3\.  \`ChatOverlay\` abre sem tirar o Stage  
4\.  Clica “Convidar para call”  
5\.  Outro lado recebe \`ConsentOverlay\`  
6\.  Aceitou → entra em \`private\_call\` (nova Session)  
7\.  Ao terminar call → volta para \`/party\` (estado preservado)

4.3 Flow C — Party → Sniffers (descoberta)  
\------------------------------------------

\*   Party continua sendo “casa”, mas Sniffers é um modo próprio:  
    \*   vai para \`/sniffers\`  
    \*   grid grande, filtros, chat/invite igual

4.4 Flow D — Random (roleta)  
\----------------------------

\*   entra em \`/random\`  
\*   fila → match → call  
\*   Skip sempre visível  
\*   Report/Block sempre visível

4.5 Flow E — Live Public (palco)  
\--------------------------------

\*   entra em \`/live/:room\_id\`  
\*   stage \+ chat público  
\*   não mistura com Party presence rail

\* \* \*

5\) Telas detalhadas (layout \+ componentes \+ estados)  
\====================================================

5.1 Tela: \*\*Broadcast Viewer\*\* (\`/\`)  
\------------------------------------

\*\*Objetivo:\*\* assistir sem atrito.

\*\*Stage\*\*

\*   \`MainPlayer\` (VVTV / HLS)

\*\*TopBar\*\*

\*   Logo (clica volta pra \`/\`)  
\*   Botão \*\*Entrar\*\*  
\*   Indicador “ao vivo agora” (opcional)

\*\*Overlays\*\*

\*   \`LoginModal\` (ou redirect externo e volta)

\*\*Estados\*\*

\*   Loading: skeleton do player \+ “conectando ao canal”  
\*   Erro no stream: “modo emergência” (fallback: loop/backup) \+ botão “tentar novamente”

\*\*Regra pós-login\*\*

\*   sucesso → \`/party\` (sem exceção, salvo deep link)

\* \* \*

5.2 Tela: \*\*Party Mode\*\* (\`/party\`)  
\-----------------------------------

\*\*Objetivo:\*\* presença viva \+ chat paralelo \+ convite para call, com VVTV como ambiente.

\*\*Stage (default)\*\*

\*   \`MainPlayer\` (VVTV / canal principal)  
\*   \`AudioPlayer\` (música do Party) \_(se habilitado)\_

\*\*Rail/Drawer\*\*

\*   \`PresenceRail\` (thumbnails WebRTC, \*\*sem áudio\*\*)  
\*   toggle: “Minha cam on/off”  
\*   contagem “X online agora”

\*\*Overlays\*\*

\*   \`ChatOverlay\` (privado 1–1)  
\*   \`ModerationSheet\` (report/block)  
\*   \`ConsentOverlay\` (quando chega convite)

\*\*Interações\*\*

\*   Tap em tile → \`ProfileCard\` (mini) com ações:  
    \*   \*\*Chat (texto)\*\*  
    \*   \*\*Convidar para call\*\*  
    \*   Report/Block

\*\*Estados\*\*

\*   Presence “sem fantasmas”: tile some rápido quando offline  
\*   Rede ruim: banner “conexão instável” \+ reattempt automático  
\*   Camera privacy: “cam desligada” com CTA para ligar

\*\*Importante (lei do Party)\*\*

\*   Presence aqui é “estética / agora”, não performer monetizado.

\* \* \*

5.3 Tela: \*\*Sniffers / Grid Discovery\*\* (\`/sniffers\`)  
\-----------------------------------------------------

\*\*Objetivo:\*\* descoberta visual “grindr/sniffies” com chat e convite.

\*\*Stage\*\*

\*   \`GridDiscovery\` (mosaico grande)

\*\*Rail/Drawer\*\*

\*   \`FiltersPanel\` (online, tags, idioma, etc.)

\*\*Overlays\*\*

\*   \`ChatOverlay\` (reuso total)  
\*   \`ModerationSheet\`

\*\*Interações\*\*

\*   Tap em tile → ProfileCard → Chat / Invite  
\*   Scroll infinito com paginação

\*\*Estados\*\*

\*   Vazio: “ninguém online nesse filtro” \+ botão limpar filtros  
\*   Carregando: skeleton grid

\* \* \*

5.4 Tela: \*\*Random\*\* (\`/random\`)  
\--------------------------------

\*\*Objetivo:\*\* match 1–1 com skip imediato, sem tropeço e com anti-abuso visível.

\*\*Stage\*\*

\*   \`CallUI\` (vídeo 1–1)

\*\*Rail/Drawer (opcional)\*\*

\*   \`QueuePanel\` (preferências, idioma, tags, “em fila”)

\*\*Overlay\*\*

\*   \`MatchOverlay\` (Searching / Connecting / Active / Cooldown)  
\*   \`ModerationSheet\` sempre acessível

\*\*Controles fixos (sempre na tela)\*\*

\*   \*\*Skip\*\*  
\*   Mute cam/mic  
\*   Report/Block

\*\*Máquina de estados\*\*

\*   Idle → Searching → Found → Connecting → Active → (Skip/End) → Searching  
\*   Cooldown pode entrar a qualquer momento por policy

\* \* \*

5.5 Tela: \*\*Invite Private Call\*\* (\`/call/:token\`)  
\--------------------------------------------------

\*\*Objetivo:\*\* consentimento explícito (não entra “do nada”).

\*\*Stage\*\*

\*   Preview (opcional) \+ fundo neutro

\*\*Overlay obrigatório\*\*

\*   \`ConsentOverlay\`:  
    \*   Aceitar  
    \*   Recusar  
    \*   Report/Block

\*\*Após aceitar\*\*

\*   Vai para \`CallUI\` (Session \`private\_call\`)

\*\*Após recusar\*\*

\*   Se logado: volta \`/party\`  
\*   Se anônimo: volta \`/\` (broadcast)

\* \* \*

5.6 Tela: \*\*Meeting\*\* (\`/meeting/:room\_id\`)  
\-------------------------------------------

\*\*Objetivo:\*\* sala N com grid e chat.

\*\*Stage\*\*

\*   \`MeetingGrid\` (tiles, speaking indicator)

\*\*Rail/Drawer\*\*

\*   \`RoomInfoPanel\` (participantes, permissões)

\*\*Overlay\*\*

\*   \`ChatOverlay\` (scope room)  
\*   \`ModerationSheet\`

\* \* \*

5.7 Tela: \*\*Live Public Room\*\* (\`/live/:room\_id\`)  
\-------------------------------------------------

\*\*Objetivo:\*\* palco \+ chat público. Separado do Party.

\*\*Stage\*\*

\*   \`PerformerStagePlayer\` (live)

\*\*Rail\*\*

\*   \`RoomChat\` (público)  
\*   (Futuro) \`TipsPanel\`

\*\*Overlay\*\*

\*   \`ModerationSheet\` reforçado (rate limit, slow mode, etc.)

\*\*Regra de separação\*\*

\*   Não aparece \`PresenceRail\` do Party aqui.

\* \* \*

5.8 Tela: \*\*Mirror\*\* (\`/mirror\`)  
\--------------------------------

\*\*Objetivo:\*\* self view, com “abrir para…” criando nova Session.

\*\*Stage\*\*

\*   \`MirrorView\` (camera local)

\*\*Controles\*\*

\*   cam/mic  
\*   “Abrir para…” (gera nova Session; nunca muda intenção silenciosamente)

\* \* \*

6\) Transições sem tropeço (o que a UI faz em cada troca)  
\========================================================

6.1 Padrão de transição (universal)  
\-----------------------------------

Quando o usuário muda de modo:

1\.  UI mostra \`TransitionLayer\` (fade/crossfade/placeholder)  
2\.  dispara \`POST /sessions/transition\` (nova Session)  
3\.  recebe endpoints novos  
4\.  reconecta o media  
5\.  remove o placeholder

6.2 O que acontece com o Broadcast durante Calls?  
\-------------------------------------------------

Pra não travar decisão agora, isso vira \*\*policy do modo\*\* (config):

\*   \*\*Default recomendado:\*\*  
    \*   Em Party: broadcast sempre tocando  
    \*   Ao entrar em call (private/random/meeting): broadcast \*\*fica em background\*\* (ou pausado), com \*\*audio duck\*\* e retorno fácil ao Party  
\*   \*\*Config opcional:\*\*  
    \*   “Broadcast always-on” (continua tocando em PiP durante calls)  
    \*   “Broadcast pause on call” (mais simples e limpo)

O importante é: a UI não reescreve nada — só muda \*\*a policy\*\*.

\* \* \*

7\) Componentes reutilizáveis (sem duplicar UI)  
\==============================================

\*   \`MainPlayer\`  
\*   \`AudioPlayer\`  
\*   \`VideoTile\`  
\*   \`PresenceRail\`  
\*   \`GridDiscovery\`  
\*   \`ChatOverlay\`  
\*   \`MatchOverlay\`  
\*   \`CallUI\`  
\*   \`MeetingGrid\`  
\*   \`RoomChat\`  
\*   \`ConsentOverlay\`  
\*   \`ModerationSheet\`  
\*   \`ModeRouter\`  
\*   \`TransitionLayer\`

Cada modo é só \*\*composição\*\*:

\*   Party \= MainPlayer \+ AudioPlayer \+ PresenceRail \+ ChatOverlay  
\*   Sniffers \= GridDiscovery \+ FiltersPanel \+ ChatOverlay  
\*   Random \= CallUI \+ MatchOverlay \+ SkipControls  
\*   Live \= PerformerStagePlayer \+ RoomChat  
\*   etc.

\* \* \*

8\) “Mapa visual” (texto) — árvore do Shell por modo  
\===================================================

\*\*Shell\*\*

\*   TopBar  
\*   Stage  
\*   Rail/Drawer  
\*   Overlays

\*\*/ (Broadcast)\*\*

\*   Stage: MainPlayer  
\*   Rail: (none)  
\*   Overlays: LoginModal, Toasts

\*\*/party\*\*

\*   Stage: MainPlayer \+ (AudioPlayer)  
\*   Rail: PresenceRail  
\*   Overlays: ChatOverlay, ConsentOverlay, ModerationSheet

\*\*/sniffers\*\*

\*   Stage: GridDiscovery  
\*   Rail: FiltersPanel  
\*   Overlays: ChatOverlay, ModerationSheet

\*\*/random\*\*

\*   Stage: CallUI  
\*   Rail: QueuePanel (opcional)  
\*   Overlays: MatchOverlay, ModerationSheet

\*\*/call/:token\*\*

\*   Stage: (preview/blank)  
\*   Overlays: ConsentOverlay → CallUI

\*\*/meeting/:room\*\*

\*   Stage: MeetingGrid  
\*   Rail: RoomInfoPanel  
\*   Overlays: ChatOverlay, ModerationSheet

\*\*/live/:room\*\*

\*   Stage: PerformerStagePlayer  
\*   Rail: RoomChat  
\*   Overlays: ModerationSheet

\*\*/mirror\*\*

\*   Stage: MirrorView  
\*   Overlays: (optional) “Open to…” sheet

\* \* \*

Se você quiser, no próximo passo eu fecho isso com \*\*um “mapa de telas em formato checklist de QA”\*\* (tipo: “ao clicar X deve abrir Y sem trocar o Stage”, “ao logar deve cair no Party”, “skip não navega”), e também um \*\*pacote de microcopy\*\* (textos dos botões/toasts) pra ficar consistente e com vibe Voulezvous.

\#\# Prompt:  
sim

\#\# Response:  
Fechado 😄✅ Vou te entregar os dois pacotes: \*\*(1) mapa de telas em formato QA\*\* \+ \*\*(2) microcopy consistente\*\*. Tudo ainda DRY (mesmos componentes/overlays) e respeitando a regra: \*\*login → Party\*\*.

\* \* \*

1\) Mapa de telas — Checklist de QA (comportamento verificável)  
\==============================================================

A) Broadcast Viewer — \`/\`  
\-------------------------

\*\*A1. Player\*\*

\*    Ao abrir \`/\`, \`MainPlayer\` inicia (ou tenta iniciar) o stream automaticamente.  
\*    Se falhar, aparece estado de erro com \*\*“Tentar novamente”\*\* sem quebrar a página.

\*\*A2. Login\*\*

\*    Botão \*\*Entrar\*\* abre fluxo de login (modal ou redirect).  
\*    Login bem-sucedido → navega para \*\*\`/party\`\*\*.  
\*    Login falhou/cancelou → volta para \`/\` sem perder o estado do player (ou re-tenta sem tela branca).

\*\*A3. Deep link\*\*

\*    Se o usuário abre \`/call/:token\` deslogado e faz login, ele \*\*continua\*\* para o fluxo do convite (não vai para \`/party\` no meio).

\* \* \*

B) Party Mode — \`/party\`  
\------------------------

\*\*B1. Composição\*\*

\*    Stage: \`MainPlayer\` (broadcast) está visível.  
\*    \`PresenceRail\` aparece (desktop) ou está acessível via drawer (mobile).  
\*    \`ChatOverlay\` existe mas \*\*não ocupa\*\* o Stage por padrão (só abre quando solicitado).

\*\*B2. Presence\*\*

\*    Lista mostra usuários “online agora” (sem fantasmas).  
\*    Ao ficar offline (sem heartbeat), tile some rapidamente.  
\*    Toggle “Minha cam” liga/desliga presença visual sem travar o Stage.

\*\*B3. Abrir perfil e chat\*\*

\*    Clicar/tocar em um tile abre \`ProfileCard\` (mini).  
\*    Clicar \*\*Chat\*\* abre \`ChatOverlay\` sem navegar para outra rota.  
\*    Fechar chat retorna ao Party sem reset.

\*\*B4. Invite to call\*\*

\*    No \`ChatOverlay\`, ação \*\*Convidar para call\*\* dispara convite.  
\*    O outro usuário recebe \`ConsentOverlay\`.  
\*    Se aceito: cria nova Session e entra em \`CallUI\` sem “tropeço” (placeholder/crossfade).  
\*    Se recusado: ambos voltam ao estado anterior com feedback (toast).

\*\*B5. Moderação\*\*

\*    \`Report/Block\` está acessível em ProfileCard e Chat.  
\*    Ao bloquear alguém, ele desaparece das listas e o chat fecha com confirmação.

\* \* \*

C) Sniffers/Grid — \`/sniffers\`  
\------------------------------

\*\*C1. Composição\*\*

\*    Stage: \`GridDiscovery\` (mosaico grande).  
\*    \`FiltersPanel\` acessível.  
\*    \`ChatOverlay\` é o mesmo do Party (sem telas duplicadas).

\*\*C2. Interações\*\*

\*    Tap/click tile → ProfileCard → Chat / Invite.  
\*    Filtros persistem durante a sessão (não resetam ao abrir/fechar chat).  
\*    Estado vazio: mostra mensagem \+ botão \*\*Limpar filtros\*\*.

\* \* \*

D) Random — \`/random\`  
\---------------------

\*\*D1. Estados\*\*

\*    Entrar em \`/random\` mostra estado inicial (Idle) com CTA \*\*Começar\*\*.  
\*    Começar → Searching (fila) via \`MatchOverlay\`.  
\*    Match found → Connecting → Active (CallUI).

\*\*D2. Skip\*\*

\*    Botão \*\*Pular\*\* sempre visível durante Active.  
\*    Ao pular, encerra Session atual e volta a Searching \*\*sem navegar\*\* para outra página.  
\*    Cooldown (se aplicável) bloqueia reentrada com timer e explicação.

\*\*D3. Segurança\*\*

\*    Report/Block sempre visível.  
\*    Bloquear durante Random interrompe a sessão e impede rematch.

\* \* \*

E) Invite Call — \`/call/:token\`  
\-------------------------------

\*\*E1. Consent\*\*

\*    Tela abre direto no \`ConsentOverlay\` (sem entrar em call automaticamente).  
\*    Aceitar → CallUI (Session \`private\_call\`).  
\*    Recusar:  
    \*    se logado → volta \`/party\`  
    \*    se não logado → volta \`/\`

\*\*E2. Erros\*\*

\*    Token inválido/expirado: mostra “convite expirou” \+ CTA voltar ao Party (ou Broadcast se deslogado).

\* \* \*

F) Meeting — \`/meeting/:room\_id\`  
\--------------------------------

\*\*F1. Composição\*\*

\*    Stage: \`MeetingGrid\`  
\*    Rail/drawer: \`RoomInfoPanel\`  
\*    ChatOverlay em scope room.

\*\*F2. Permissões\*\*

\*    Host vê controles extra (se habilitado).  
\*    Usuário comum não vê ações de host.

\* \* \*

G) Live Public — \`/live/:room\_id\`  
\---------------------------------

\*\*G1. Separação de intenção\*\*

\*    Stage: \`PerformerStagePlayer\`  
\*    Rail: \`RoomChat\` público  
\*    Não existe \`PresenceRail\` do Party aqui.

\*\*G2. Rate limit\*\*

\*    Chat aplica rate limit (mensagem de “devagar” quando necessário).

\* \* \*

H) Mirror — \`/mirror\`  
\---------------------

\*\*H1. Ação explícita\*\*

\*    Mirror mostra self view local.  
\*    CTA \*\*Abrir para…\*\* sempre cria nova Session (nunca “vira público” silenciosamente).

\* \* \*

2\) Microcopy (textos) — consistente e sem “dashboard”  
\=====================================================

A) TopBar / Botões  
\------------------

\*   \*\*Entrar\*\*  
\*   \*\*Voltar pro Party\*\*  
\*   \*\*Minha cam: ligada / desligada\*\*  
\*   \*\*Chat\*\*  
\*   \*\*Convidar pra call\*\*  
\*   \*\*Pular\*\*  
\*   \*\*Aceitar / Recusar\*\*  
\*   \*\*Reportar\*\*  
\*   \*\*Bloquear\*\*

B) Toasters (feedback curto)  
\----------------------------

\*   Login:  
    \*   “Logado ✅ Bem-vindo ao Party.”  
    \*   “Login cancelado.”  
    \*   “Não deu certo. Tenta de novo.”  
\*   Presence:  
    \*   “Sua cam está ligada.”  
    \*   “Sua cam está desligada.”  
    \*   “Conexão instável… ajustando.”  
\*   Chat/Call:  
    \*   “Convite enviado.”  
    \*   “Convite aceito ✅”  
    \*   “Convite recusado.”  
    \*   “Chamando…”  
    \*   “Chamada encerrada.”  
\*   Moderação:  
    \*   “Report recebido. Obrigado.”  
    \*   “Usuário bloqueado.”  
    \*   “Você não verá mais essa pessoa.”

C) Estados vazios / erros  
\-------------------------

\*   Sniffers vazio:  
    \*   “Ninguém online com esses filtros.”  
    \*   CTA: “Limpar filtros”  
\*   Convite inválido:  
    \*   “Esse convite expirou.”  
    \*   CTA: “Voltar ao Party” / “Voltar pra TV”  
\*   Random cooldown:  
    \*   “Pausa rápida pra segurança.”  
    \*   “Você pode tentar de novo em {mm:ss}.”

D) Textos “soft” (vibe VV)  
\--------------------------

\*   Party header (opcional):  
    \*   “Agora.”  
    \*   “Quem tá aqui.”  
    \*   “Vem conversar.”

\* \* \*

Quer que eu finalize em formato “doc pronto pra repo”?  
\------------------------------------------------------

Se você quiser, eu reempacoto isso como:

\*   \`ui-spec.md\` (telas \+ composição)  
\*   \`qa-checklist.md\` (os checkboxes)  
\*   \`copy.json\` (microcopy centralizado por keys, pra não repetir string em código)

Só me diz: você quer a vibe dos textos \*\*mais “sexy minimal”\*\* (bem curta) ou \*\*mais “friendly”\*\* (com 1 frase a mais)?

\#\# Prompt:  
Quais são os backends necessários para esse grande projeto

\#\# Response:  
Thought for 14s

Boa — pensando nesse \*\*plano grande como uma plataforma única\*\*, os “backends necessários” na prática são \*\*módulos/serviços\*\* (alguns podem virar microserviços separados, outros podem ser “módulos” no mesmo runtime). Vou listar em camadas pra ficar bem claro e \*\*sem duplicação de lógica\*\*. 🚀

\* \* \*

1\) Control Plane (o cérebro: estado, presença, regras, transições)  
\------------------------------------------------------------------

\#\#\# 1\) \*\*Identity & Auth\*\*

\*   Login, sessão, tokens, device/session  
\*   Flags: idade/consent, bans, reputation/quarantine  
\*   RBAC leve (admin, mod, user)

\#\#\# 2\) \*\*OwnerCard / Policy Service (Cartão do Dono)\*\*

\*   Armazena/versão políticas (editorial \+ operacionais)  
\*   “Policy snapshot” aplicado na criação de sessões e decisões  
\*   Audit trail de mudanças

\#\#\# 3\) \*\*Presence Service (WebSocket \+ heartbeat)\*\*

\*   “Who’s online now” por modo (party/grid/random/etc.)  
\*   TTL/heartbeat anti-fantasma  
\*   Buckets/shards (pra escalar)

\#\#\# 4\) \*\*Session Orchestrator\*\*

\*   Cria/encerra/transition sessions (\*\*troca de modo \= nova session\*\*)  
\*   Emite “endpoints” (tokens/urls) via MediaProvider  
\*   Lineage: \`parent\_session\_id\`, \`intent\_label\`

\#\#\# 5\) \*\*Chat Service\*\*

\*   Threads: private 1–1, room, performer\\\_room  
\*   WebSocket por thread  
\*   Rate-limit \+ report hooks

\#\#\# 6\) \*\*Discovery Service\*\*

\*   Grid/sniffers: ranking, filtros, segmentos  
\*   Basicamente leitura sobre Presence \+ perfil \+ policy (não duplicar “lista de online”)

\#\#\# 7\) \*\*Matching Service (Random)\*\*

\*   Fila/join/skip/cooldown  
\*   Anti-abuso (quarantine, throttling)  
\*   Só decide match; quem cria a call é o Session Orchestrator

\#\#\# 8\) \*\*Moderation & Safety\*\*

\*   Reports, block, ban, shadow/quarantine  
\*   Consent handshake (private call)  
\*   Regras “separação de ecossistemas” (Party ≠ Live Public ≠ Random)

\#\#\# 9\) \*\*Audit Log Service\*\*

\*   Registro explicável de decisões (especialmente curador e moderação)  
\*   Útil pra “por que isso entrou / por que isso foi bloqueado”

\* \* \*

2\) Media Plane (vídeo/áudio: WebRTC/HLS/ingest) — \*\*sempre atrás de uma interface\*\*  
\-----------------------------------------------------------------------------------

Aqui entra o ponto DRY: você NÃO espalha “código de WebRTC” pelo produto. Você cria um backend \*\*Media Control\*\* com uma interface única.

\#\#\# 10\) \*\*Media Control / MediaProvider Adapter\*\*

\*   API interna tipo:  
    \*   \`createWebRTCEndpoints(session, profile)\`  
    \*   \`createTurnCredentials(user/session)\`  
    \*   \`createBroadcastOutput(...)\`  
    \*   \`revokeEndpoints(session)\`  
\*   Implementações plugáveis (Cloudflare / LiveKit / etc.)  
\*   Perfis reusáveis: \`presence\_thumb\_no\_audio\`, \`random\_fast\_connect\`, \`meeting\_grid\`, etc.

\#\#\# 11\) \*\*SFU \+ TURN\*\*

\*   Necessário pra calls (random/private/meeting) e thumbs de presença com confiabilidade (NAT/4G)  
\*   Pode ser gerenciado (ideal) ou self-host

\#\#\# 12\) \*\*Broadcast Delivery (HLS)\*\*

\*   Player do canal principal (VVTV / Party main content)  
\*   Ingest/egress se tiver live public

\> Resumo: \*\*MediaProvider \+ SFU/TURN \+ HLS\*\* cobrem tudo de mídia.

\* \* \*

3\) VVTV / LAB Plane (o “motor” 24/7 que roda no seu Mac mini)  
\-------------------------------------------------------------

Isso é “backend” também, só que local.

\#\#\# 13\) \*\*Planner (Plan Service local)\*\*

\*   Gera PLANOS (intenções) sem baixar  
\*   Aplica OwnerCard e regras de variedade

\#\#\# 14\) \*\*Fetcher / Downloader (T-4h)\*\*

\*   Confirma disponibilidade e baixa só o necessário  
\*   Fallbacks e reservas

\#\#\# 15\) \*\*Prepare & QA Pipeline\*\*

\*   Container fix, normalização de áudio, checks de integridade/qualidade  
\*   Reprova/troca por reservas

\#\#\# 16\) \*\*Broadcast Orchestrator \+ Transmitter\*\*

\*   Fila com folga (\~1h)  
\*   Modo emergência (fallback)  
\*   Publica HLS (ou empurra pro provider)

\#\#\# 17\) \*\*Curator (local)\*\*

\*   Sugere trocas e aplica dentro de limites  
\*   Tudo auditado

\#\#\# 18\) \*\*Lab Agent (ponte LAB ↔ nuvem)\*\*

\*   “Comandos” vindos do painel (start/stop, reload policies, rotate playlists)  
\*   Envia status/telemetria resumida

\* \* \*

4\) Ops Plane (sem isso, o grande projeto vira imprevisível)  
\-----------------------------------------------------------

\#\#\# 19\) \*\*Telemetry / Analytics Ingestion\*\*

\*   Funil (viewer→login→party→chat→call)  
\*   QoS (quedas, RTT/jitter em alto nível)  
\*   Saúde do broadcast (buffer, emergências)

\#\#\# 20\) \*\*Status / Monitoring API\*\*

\*   Página de status  
\*   Alertas (broadcast caiu, presence travou, matching degradou)

\#\#\# 21\) \*\*Admin/Backoffice\*\*

\*   Moderação (reports, ban, appeals)  
\*   Ajustes do OwnerCard (versionados)  
\*   Gestão de rooms (live public) se existir

\* \* \*

5\) Storage mínimo (os “bancos” por natureza do dado)  
\----------------------------------------------------

Pra não misturar tudo num balaio:

\*   \*\*Persistente (relacional)\*\*: Users, Sessions, Chat metadata, OwnerCard, AuditLog  
\*   \*\*Ephemeral/rápido\*\*: Presence TTL, filas do Random, rate limits (Redis-like)  
\*   \*\*Eventos/Fila\*\*: Jobs de moderação/telemetria/rotinas (queue)  
\*   \*\*Objetos\*\*: Assets preparados, thumbnails, logs compactados (object storage)

\* \* \*

6\) O mínimo pra ligar (sem se enganar)  
\--------------------------------------

Se você quer \*\*rodar VVTV \+ login→Party \+ chat+invite\*\*, dá pra começar com estes backends:

\*\*Essenciais (MVP-1 \+ Party)\*\*

1\.  Identity & Auth  
2\.  OwnerCard/Policy  
3\.  Presence (WS)  
4\.  Session Orchestrator  
5\.  Chat  
6\.  Moderation básico (report/block/ban)  
7\.  MediaProvider \+ TURN (pra calls)  
8\.  LAB: Planner \+ Downloader \+ QA \+ Transmitter \+ Agent  
9\.  Telemetry/Status mínimo

\*\*Depois (pra completar o plano grande)\*\*

\*   Discovery (Sniffers)  
\*   Matching (Random)  
\*   Live Public (rooms \+ chat público \+ moderação reforçada)  
\*   Admin completo \+ analytics mais rico

\* \* \*

Se você me disser \*\*“Cloudflare-first” ou “agnóstico (plugável)”\*\*, eu te devolvo a mesma lista já \*\*mapeada em implementação\*\* (ex.: quais módulos viram Durable Objects/Workers, quais rodam no LAB, quais ficam como serviços separados), sem mudar o desenho modular.

\#\# Prompt:  
 LAB First for Cost, Cloudflare para qualidade mas tomar cuidado com cost

\#\# Response:  
Thought for 40s

Perfeito — \*\*LAB-first pra custo\*\* e \*\*Cloudflare pra qualidade\*\* dá um desenho bem forte, \_desde que você trate “Cloudflare \= controle \+ qualidade onde dói”\_ e não jogue o \*\*24/7 inteiro\*\* em serviços cobrados por consumo sem guardrail. Bora no mapa. ⚙️💸

\* \* \*

1\) O que roda onde (recomendação “LAB-first”)  
\---------------------------------------------

\#\#\# A) \*\*Sempre no LAB (Mac mini) — porque é o seu “custo fixo”\*\*

\*\*VVTV core (produção 24/7)\*\*

\*   Descoberta/validação de fontes \+ geração de \*\*PLANOS\*\*  
\*   Download T-4h \+ fallback/reservas  
\*   Preparação/QA (contêiner, áudio, checks)  
\*   Queue \+ transmissão 24/7 \+ modo emergência  
\*   Curador local (sugere/aplica dentro de limites)  
\*   Logs completos e evidências “pesadas” (sem vazar nada)

✅ Isso mantém o “canal sempre no ar” sem custo variável por audiência.

\* \* \*

\#\#\# B) \*\*Cloudflare (sempre) — control plane leve e confiável\*\*

\*\*O que vale muito colocar na Cloudflare logo de cara\*\*

\*   \*\*Auth / sessão / flags\*\* (idade/consent, ban/quarantine)  
\*   \*\*Presence (online agora)\*\* \+ WebSocket  
\*   \*\*Chat\*\* (threads privadas/rooms)  
\*   \*\*Session Orchestrator\*\* (criar/transition/end) \+ emissão de endpoints via MediaProvider  
\*   \*\*Moderation básica\*\* (report/block/ban)  
\*   \*\*Status/monitoramento\*\* \+ relatórios “resumidos”

💡 Aqui o custo tende a ser previsível se você mantiver tudo “leve”. Workers cobram por \*\*CPU time\*\* e você consegue colocar \*\*CPU limits\*\* pra evitar conta disparar. \[Cloudflare Docs+1\](https://developers.cloudflare.com/workers/platform/pricing/?utm\_source=chatgpt.com)    
Durable Objects cobram \*\*compute \+ storage\*\* (e dá pra reduzir custo de WebSocket com \*\*hibernation\*\*). \[Cloudflare Docs+2GitHub+2\](https://developers.cloudflare.com/durable-objects/platform/pricing/?utm\_source=chatgpt.com)

\* \* \*

2\) Onde a Cloudflare entrega qualidade — e onde o custo pode explodir  
\---------------------------------------------------------------------

\#\#\# A) \*\*WebRTC (calls, meeting, thumbs de presença)\*\*

Cloudflare Realtime (SFU \+ TURN) é bom pra qualidade operacional e NAT/4G. O pricing é por \*\*egress em GB\*\* com \*\*1.000 GB free\*\* compartilhado entre SFU e TURN, depois \*\*US$ 0,05/GB\*\*. \[Cloudflare Docs+1\](https://developers.cloudflare.com/realtime/sfu/pricing/?utm\_source=chatgpt.com)

✅ Bom custo/benefício \*\*se\*\* você:

\*   fizer thumbs de presença \*\*sem áudio\*\*, resolução baixa, FPS baixo  
\*   limitar quantos tiles “ativos” o usuário pode ver ao mesmo tempo  
\*   aplicar \*\*degradação automática\*\* em rede ruim

⚠️ Onde estoura: “Party com 30 tiles em 30 fps pra todo mundo”. Aí vira serra elétrica no egress.

\*\*Sugestão prática:\*\*

\*   Presence cams \= \*\*perfil “thumb\\\_low”\*\* (ex.: 160p/180p, 5–8 fps, sem áudio, bitrate cap)  
\*   Calls \= \*\*perfil “call\\\_standard”\*\* (cap 720p)  
\*   Meeting \= “grid\\\_adaptive” (cap por número de participantes)

Tudo isso vira policy/config, não código duplicado.

\* \* \*

\#\#\# B) \*\*Broadcast/streaming (VVTV 24/7)\*\*

Cloudflare Stream é ótimo, mas o modelo é \*\*minutos armazenados \+ minutos entregues\*\*; ingest e encoding são “free”, mas “delivered minutes” é literalmente \_viewers × minutos\_. \[Cloudflare Docs+1\](https://developers.cloudflare.com/stream/pricing/?utm\_source=chatgpt.com)

✅ Use Cloudflare Stream quando:

\*   você quiser qualidade global com operação “zero dor”  
\*   você estiver ok com custo variável por audiência  
\*   for “evento/room” (Live Public) com duração finita

⚠️ Para \*\*TV 24/7\*\*, se a audiência cresce, “minutos entregues” cresce sem dó. Então, \*\*LAB-first\*\* aqui faz muito sentido: \*\*deixa o 24/7 no LAB\*\* e só migra para Stream quando você estiver “comprando” essa elasticidade.

\* \* \*

3\) Guardrails de custo (o kit “não toma susto na fatura”)  
\---------------------------------------------------------

\#\#\# 3.1 Limites e kill-switches (obrigatórios)

\*\*Workers\*\*

\*   setar \*\*CPU limits\*\* e timeouts pra evitar runaway/DoW (denial-of-wallet). \[Cloudflare Docs\](https://developers.cloudflare.com/workers/platform/pricing/?utm\_source=chatgpt.com)

\*\*Durable Objects\*\*

\*   usar WebSocket \*\*Hibernation\*\* pra não pagar “duração” desnecessária quando tá ocioso. \[GitHub+1\](https://github.com/cloudflare/cloudflare-docs/blob/production/src/content/docs/durable-objects/platform/pricing.mdx?utm\_source=chatgpt.com)  
\*   TTL agressivo pra Presence (anti-fantasma \+ menos churn)

\*\*Realtime\*\*

\*   caps por perfil (thumb/call/meeting)  
\*   limitar tiles ativos (ex.: só os 12 visíveis na tela conectam de verdade; o resto é “paused”)  
\*   áudio sempre off nas thumbs

\*\*Stream (se usar)\*\*

\*   só habilitar pra modos “finito” (Live Public) ou quando houver gatilho de escala  
\*   metas de audiência \+ orçamento mensal antes de ligar 24/7 nele

\* \* \*

4\) “Cost Modes” no Cartão do Dono (controle sem mexer no código)  
\----------------------------------------------------------------

Eu colocaria um switch global de custo/qualidade no OwnerCard, tipo:

\*   \*\*COST\\\_LOW (padrão LAB-first)\*\*  
    \*   Party thumbs bem econômicos  
    \*   Broadcast 24/7 no LAB  
    \*   Telemetria amostrada  
\*   \*\*BALANCED\*\*  
    \*   thumbs um pouco melhores  
    \*   calls com SFU sempre  
    \*   mais métricas de QoS  
\*   \*\*QUALITY\\\_MAX\*\*  
    \*   thumbs maiores / fps melhor  
    \*   meeting mais estável  
    \*   (opcional) Stream para eventos / picos

Isso deixa você ajustar o “slider” sem refatorar arquitetura.

\* \* \*

5\) Minha recomendação final (bem direta)  
\----------------------------------------

\*\*Começa assim (barato e sólido):\*\*

1\.  \*\*VVTV 24/7 inteiro no LAB\*\* (custo fixo, controle total)  
2\.  \*\*Cloudflare só no control plane\*\* (auth/presence/chat/sessions/moderação/status)  
3\.  \*\*Cloudflare Realtime apenas para calls/meeting/random\*\* e thumbs do Party com \*\*perfil ultra-low\*\*

\*\*Evolui assim (qualidade sob demanda):\*\*    
4\) Se Live Public crescer: avalia \*\*Stream\*\* pra rooms/eventos (não para a TV 24/7 ainda)    
5\) Só joga o 24/7 no Stream quando você tiver \*\*teto de orçamento\*\* e quiser “escala sem dor”. \[Cloudflare Docs\](https://developers.cloudflare.com/stream/pricing/?utm\_source=chatgpt.com)

\* \* \*

Se você topar, eu fecho isso num \*\*mapa operacional\*\* (tipo “matriz por modo”):

\*   Party: o que fica no LAB vs Cloudflare, perfis de mídia, limites  
\*   Random/Private/Meeting: perfis, caps, custo esperado por hora  
\*   Live Public: quando usar Stream vs quando não usar

E já deixo \*\*valores default de caps\*\* (resolução/fps/bitrate/tile limit) no estilo “COST\\\_LOW”.

\#\# Prompt:  
matriz

\#\# Response:  
Thought for 36s

Fechou — aqui vai a \*\*matriz LAB-first (custo)\*\* \+ \*\*Cloudflare (qualidade)\*\*, já com \*\*guardrails anti-conta-surpresa\*\*. Vou assumir \*\*COST\\\_LOW\*\* como default e tudo configurável via “Cartão do Dono / Policy”. 💸⚙️

\* \* \*

Regras de custo (valem pra tudo)  
\--------------------------------

\*   \*\*Broadcast 24/7 (VVTV)\*\*: \*\*LAB\*\* por padrão. Cloudflare Stream só entra quando você aceitar “custo variável por audiência” (minutos entregues). \[Cloudflare Docs+1\](https://developers.cloudflare.com/stream/pricing/?utm\_source=chatgpt.com)  
\*   \*\*Realtime (calls / thumbs / meeting)\*\*: cobrar por \*\*egress GB\*\* (SFU+TURN juntos, com free tier e depois $/GB). \[Cloudflare Docs\](https://developers.cloudflare.com/realtime/sfu/pricing/?utm\_source=chatgpt.com)  
\*   \*\*Presence/Chat\*\* em Durable Objects: usar \*\*WebSocket Hibernation\*\* pra derrubar custo de duração. \[Cloudflare Docs+1\](https://developers.cloudflare.com/durable-objects/platform/pricing/?utm\_source=chatgpt.com)  
\*   \*\*Workers\*\*: custo é \*\*CPU time\*\* (não “espera por I/O”), então evite loops/serializações pesadas e aplique limites. \[The Cloudflare Blog+1\](https://blog.cloudflare.com/workers-pricing-scale-to-zero/?utm\_source=chatgpt.com)

\* \* \*

Matriz por modo (LAB vs Cloudflare \+ caps COST\\\_LOW)  
\====================================================

\> \*\*Glossário rápido:\*\*    
\> \*\*CP\*\* \= Control Plane (auth/presence/chat/sessions/moderação) • \*\*MP\*\* \= Media Plane (HLS/WebRTC/SFU/TURN)

| Modo | Stage/UI dominante | LAB (o que roda local) | Cloudflare CP (sempre) | Cloudflare MP (quando usa) | \*\*COST\\\_LOW caps (defaults)\*\* | Principais “drivers” de custo | Guardrails obrigatórios |  
| \--- | \--- | \--- | \--- | \--- | \--- | \--- | \--- |  
| \*\*Broadcast (VVTV)\*\* \`/\` | \`MainPlayer\` (TV 24/7) | \*\*Tudo do broadcast\*\*: fila, folga, emergência, transmissão | status/telemetria resumida \+ policy sync | \*\*Evitar Stream\*\* por padrão (minutos entregues) \[Cloudflare Docs\](https://developers.cloudflare.com/stream/pricing/?utm\_source=chatgpt.com) | HLS no LAB; múltiplas ladders mínimas | audiência × minutos (se Stream), bandwidth do seu uplink (se LAB) | fallback local (loop/reservas), health checks, “panic mode” |  
| \*\*Party\*\* \`/party\` | TV \+ PresenceRail \+ ChatOverlay | (Opcional) nada crítico; TV segue no LAB | auth, presence, chat, sessions, moderação | \*\*Realtime SFU/TURN\*\* pras thumbs \+ calls \[Cloudflare Docs\](https://developers.cloudflare.com/realtime/sfu/pricing/?utm\_source=chatgpt.com) | \*\*Thumbs:\*\* 180p, 6–8fps, \*\*sem áudio\*\*, cap bitrate; \*\*Tiles ativos:\*\* 12 (desktop), 6 (mobile) | egress \= tiles ativos × bitrate × viewers | só conecta tiles visíveis; auto-degrade em 4G; hibernation DO \[Cloudflare Docs\](https://developers.cloudflare.com/durable-objects/examples/websocket-hibernation-server/?utm\_source=chatgpt.com) |  
| \*\*Sniffers/Grid\*\* \`/sniffers\` | \`GridDiscovery\` \+ ChatOverlay | — | discovery queries \+ presence \+ chat | thumbs via Realtime (opcional) | mesmo perfil de thumbs do Party; “hover/tap” só abre preview se necessário | egress por thumbs em massa | paginação dura \+ “connect on demand” (só quando tile entra na viewport) |  
| \*\*Random\*\* \`/random\` | \`CallUI\` \+ MatchOverlay | — | matching \+ sessions \+ moderação | \*\*Realtime SFU/TURN\*\* (recomendado) \[Cloudflare Docs\](https://developers.cloudflare.com/realtime/sfu/pricing/?utm\_source=chatgpt.com) | 480p default, 720p só em Wi-Fi; áudio ok; reconnect rápido | egress por call (tempo em call × bitrate) | cooldown/quarantine; limite de reconexões; report/block sempre visível |  
| \*\*Private Call\*\* | \`CallUI\` \+ ConsentOverlay | — | consent handshake \+ sessions \+ moderação | Realtime SFU/TURN \[Cloudflare Docs\](https://developers.cloudflare.com/realtime/sfu/pricing/?utm\_source=chatgpt.com) | 720p cap; “audio duck” do broadcast; chat paralelo opcional | egress por call | consent obrigatório; timeout; kill switch por policy |  
| \*\*Meeting\*\* | \`MeetingGrid\` \+ RoomInfo \+ Chat | — | room state \+ permissions \+ moderação | Realtime SFU/TURN \[Cloudflare Docs\](https://developers.cloudflare.com/realtime/sfu/pricing/?utm\_source=chatgpt.com) | cap por N: ex. \>6 ppl → 360p; áudio ok | egress cresce com N (grid) | limite de participantes por tier; “active speaker mode” reduz tiles recebidos |  
| \*\*Live Public (Performer)\*\* \`/live/:id\` | \`StagePlayer\` \+ chat público | (Opcional) ingest se você quiser barato/DIY | room metadata \+ chat \+ moderação reforçada | \*\*Stream\*\* (se quiser simplicidade) ou outro provider; Stream cobra por minutos entregues \[Cloudflare Docs\](https://developers.cloudflare.com/stream/pricing/?utm\_source=chatgpt.com) | 1 ladder só no início; chat slow mode | se Stream: viewers × minutos; se WebRTC: egress GB \[Cloudflare Docs\](https://developers.cloudflare.com/realtime/sfu/pricing/?utm\_source=chatgpt.com) | separar discovery do Party; rate limit/slow mode; orçamento mensal por room |  
| \*\*Mirror\*\* \`/mirror\` | self view local | local camera preview | auth \+ flags | nenhum (até “Abrir para…”) | n/a | n/a | “Abrir para…” sempre cria nova Session |

\* \* \*

Defaults COST\\\_LOW (valores que \_seguram a conta\_)  
\--------------------------------------------------

\#\#\# 1\) Presence thumbs (Party/Sniffers)

\*   \*\*Sem áudio\*\* (sempre)  
\*   \*\*180p, 6–8 fps\*\*, bitrate cap agressivo  
\*   \*\*Só 12 tiles ativos\*\* (desktop) / \*\*6\*\* (mobile)  
\*   Conecta \*\*somente\*\* tiles na viewport (resto: paused/snapshot)

\*\*Regra prática de custo (Realtime egress):\*\*    
\`GB/h ≈ 0,439 × Mbps\_total\_recebido\_por\_usuário\`    
Ex.: 12 tiles × 0,15 Mbps \= 1,8 Mbps → \*\*\~0,79 GB/h por viewer\*\*.    
Aí você multiplica por concorrência. (Realtime cobra por egress, SFU+TURN juntos). \[Cloudflare Docs\](https://developers.cloudflare.com/realtime/sfu/pricing/?utm\_source=chatgpt.com)

\#\#\# 2\) Calls (Random/Private/Meeting)

\*   Default \*\*480p\*\* (subir pra 720p só com rede boa)  
\*   “Active speaker” / limitar streams simultâneos em meeting  
\*   “Reconnect budget”: máximo de tentativas por minuto

\#\#\# 3\) Stream (se/quando usar)

Stream cobra por \*\*minutos entregues\*\* \+ \*\*minutos armazenados\*\* (ingest/encoding “free”, o custo é nesses 2 eixos). \[Cloudflare Docs+1\](https://developers.cloudflare.com/stream/pricing/?utm\_source=chatgpt.com)    
Então: \*\*TV 24/7 no Stream \= custo ∝ audiência\*\* (muito previsível, mas pode crescer rápido).

\* \* \*

Guardrails técnicos (anti “denial-of-wallet”)  
\---------------------------------------------

1\.  \*\*Budget switches no Policy (OwnerCard)\*\*  
    \*   \`COST\_LOW | BALANCED | QUALITY\_MAX\`  
\*   muda caps de thumbs/calls/meeting sem deploy  
    2\.  \*\*Workers: limitar CPU e payload\*\*    
    Workers são cobrados por CPU time — evite parse/encode pesado e aplique limites/timeout. \[The Cloudflare Blog+1\](https://blog.cloudflare.com/workers-pricing-scale-to-zero/?utm\_source=chatgpt.com)  
3\.  \*\*Durable Objects: hibernation \+ TTL\*\*    
    WebSocket Hibernation reduz custo de duração em DO para apps de WS server. \[Cloudflare Docs+1\](https://developers.cloudflare.com/durable-objects/platform/pricing/?utm\_source=chatgpt.com)    
    Presence TTL curto evita “fantasmas” e também evita objetos quentes o tempo todo.  
4\.  \*\*Realtime: cap por perfil \+ tiles visíveis\*\*    
    Realtime SFU/TURN: preço por egress com free tier e depois por GB. \[Cloudflare Docs\](https://developers.cloudflare.com/realtime/sfu/pricing/?utm\_source=chatgpt.com)    
    Você controla custo quase totalmente por:  
    \*   quantos streams o cliente recebe  
\*   bitrate/fps/resolução  
\*   quanto tempo fica conectado

\* \* \*

Próximo passo (pra ficar “implementável”)  
\-----------------------------------------

Me diz só \*\*qual é o target inicial\*\*:

\*   \*\*A)\*\* Party com thumbs \+ chat \+ invite (sem Random ainda)    
    ou  
\*   \*\*B)\*\* já incluir Random no MVP

Com isso eu te devolvo a \*\*mesma matriz\*\* com:

\*   \*\*perfis de mídia\*\* nomeados (\`thumb\_low\`, \`call\_std\`, \`meeting\_grid\_adaptive\`)  
\*   \*\*limites por dispositivo\*\* (desktop/mobile)  
\*   e um \*\*orçamento mensal\*\* em fórmulas (sem chutar número) pra você plugar num “cost dashboard”.

\#\# Prompt:  
inclui random

\#\# Response:  
Thought for 17s

Fechado — \*\*incluindo Random no MVP\*\*, aqui vai a \*\*matriz completa (LAB-first \+ Cloudflare quality com guardrails de custo)\*\* já com:

\*   \*\*perfis de mídia nomeados\*\* (pra não espalhar tuning no código),  
\*   \*\*caps por device\*\* (desktop/mobile),  
\*   \*\*fórmulas de custo\*\* (Realtime egress),  
\*   e \*\*regras anti-explosão\*\* específicas do Random (skip loops, cooldown, quarantine).

\> Base de preço que estou usando: \*\*Cloudflare Realtime SFU/TURN cobra por egress em GB\*\* (SFU+TURN somados), com \*\*free tier\*\* e depois \*\*US$0,05/GB\*\*.    
\> Durable Objects têm custo por compute/storage e vale usar \*\*WebSocket hibernation\*\* pra segurar custo.    
\> Stream cobra por minutos (não recomendo pra TV 24/7 no começo).

\* \* \*

1\) Perfis de mídia (DRY): 1 lugar pra ajustar custo/qualidade  
\=============================================================

\#\#\# \*\*thumb\\\_low\*\* (Party/Sniffers presence cams)

\*   Vídeo: \*\*180p\*\*, \*\*6–8 fps\*\*, bitrate cap \*\*120–180 kbps\*\*  
\*   Áudio: \*\*OFF\*\*  
\*   Conexão: só tiles \*\*visíveis\*\* (resto \= paused/snapshot)

\#\#\# \*\*call\\\_random\\\_fast\*\*

\*   Vídeo default: \*\*480p\*\*, \*\*20–24 fps\*\*, bitrate cap \*\*700–1000 kbps\*\*  
\*   Upgrade (só Wi-Fi \+ sinal bom): \*\*720p\*\*, \*\*24–30 fps\*\*, cap \*\*1400–2200 kbps\*\*  
\*   Áudio: Opus \*\*24–32 kbps\*\*  
\*   Reconexão: budget limitado (pra não gastar em looping)

\#\#\# \*\*call\\\_private\\\_std\*\*

\*   Parecido com \`call\_random\_fast\`, mas com tolerância maior de qualidade (menos agressivo nos caps)

\#\#\# \*\*meeting\\\_adaptive\*\*

\*   ≤4 pessoas: 540p/720p adaptativo  
\*   5–8 pessoas: \*\*360–480p\*\*  
\*   \> 8: \*\*360p\*\* \+ “active speaker” prioritário (reduz streams recebidos)

\#\#\# \*\*stage\\\_live\\\_public\*\*

\*   Idealmente HLS/Stream (custo variável por audiência, só quando você aceitar)

\* \* \*

2\) Matriz por modo (LAB vs Cloudflare \+ caps por device)  
\========================================================

Convenções  
\----------

\*   \*\*CP\*\* \= control plane (auth/presence/chat/sessions/moderação/matching)  
\*   \*\*MP\*\* \= media plane (WebRTC SFU/TURN, HLS)

\#\#\# ✅ Core sempre

\*   VVTV broadcast 24/7: \*\*LAB\*\*  
\*   CP: \*\*Cloudflare\*\* (leve, confiável)

\* \* \*

A) Broadcast / VVTV — \`/\`  
\-------------------------

\*\*Stage:\*\* \`MainPlayer (HLS)\`

\*   \*\*LAB:\*\* planner → T-4h download → QA → queue/folga → transmissão/backup  
\*   \*\*Cloudflare CP:\*\* policy sync \+ status \+ relatórios resumidos  
\*   \*\*Cloudflare MP:\*\* \*\*evitar Stream\*\* pro 24/7 por custo variável (minutos entregues)

\*\*COST\\\_LOW defaults\*\*

\*   1–2 ladders HLS (não inventar 6 ladders no começo)  
\*   modo emergência local (loop/reserva)

\*\*Drivers de custo\*\*

\*   principalmente seu uplink/infra local (previsível)

\* \* \*

B) Party — \`/party\`  
\-------------------

\*\*Stage:\*\* VVTV (HLS do LAB) \+ (música opcional)    
\*\*Rail:\*\* Presence cams \+ ChatOverlay

\*   \*\*Cloudflare CP:\*\* auth, presence (WS), chat, sessions, moderação  
\*   \*\*Cloudflare MP:\*\* WebRTC thumbs (\`thumb\_low\`) \+ calls (quando convidar)

\*\*COST\\\_LOW caps\*\*

\*   Desktop: \*\*12 tiles ativos\*\* (thumb\\\_low)  
\*   Mobile: \*\*6 tiles ativos\*\*  
\*   \*\*Áudio OFF\*\* nos thumbs (sempre)  
\*   Tiles fora da viewport: \*\*paused\*\*

\*\*Drivers de custo\*\*

\*   egress \= \`tiles\_ativos × bitrate\_thumb × viewers\_concorrentes\`

\*\*Guardrails\*\*

\*   “connect only visible tiles”  
\*   degrade automático (se rede ruim → 120p/4fps)  
\*   DO WebSocket hibernation

\* \* \*

C) Sniffers/Grid — \`/sniffers\`  
\------------------------------

\*\*Stage:\*\* grid grande \+ filtros    
\*\*Overlay:\*\* chat/invite reusável

\*   \*\*Cloudflare CP:\*\* discovery (queries) \+ presence \+ chat  
\*   \*\*Cloudflare MP:\*\* thumbs \`thumb\_low\` \*\*on-demand\*\*

\*\*COST\\\_LOW caps\*\*

\*   Desktop: \*\*16 tiles renderizados\*\*, mas \*\*apenas 12 conectados\*\*  
\*   Mobile: \*\*8 render\*\*, \*\*6 conectados\*\*  
\*   “hover/tap → connect”, senão snapshot

\*\*Guardrails\*\*

\*   paginação dura \+ viewport gating

\* \* \*

D) Random — \`/random\` ✅ (incluído)  
\----------------------------------

\*\*Stage:\*\* \`CallUI\`    
\*\*Overlay:\*\* \`MatchOverlay\`    
\*\*Botões fixos:\*\* Skip \+ Report/Block

\*   \*\*Cloudflare CP:\*\* matching \+ sessions \+ moderação \+ cooldown/quarantine  
\*   \*\*Cloudflare MP:\*\* WebRTC via SFU/TURN usando \`call\_random\_fast\`

\#\#\# COST\\\_LOW caps por device (Random)

\*\*Desktop\*\*

\*   Default: \*\*480p @ 20–24fps\*\*, cap \*\*0.7–1.0 Mbps\*\*  
\*   Upgrade: \*\*720p\*\* só Wi-Fi \+ QoS bom (cap 1.4–2.2 Mbps)  
\*   Audio: \*\*24–32 kbps\*\*

\*\*Mobile\*\*

\*   Default: \*\*360–480p\*\*, cap \*\*0.5–0.9 Mbps\*\*  
\*   Upgrade raro (só Wi-Fi excelente)  
\*   Aggressive fallback em 4G

\#\#\# Guardrails específicos do Random (anti custo \+ anti abuso)

\*   \*\*Skip cooldown\*\* progressivo:  
    \*   0–3 skips rápidos: ok  
    \*   depois: 3–10s  
    \*   se continuar: 30–60s \+ quarantine  
\*   \*\*Match minimum time\*\* (ex.: 2s antes de permitir novo skip) pra evitar spam de conexão  
\*   \*\*Reconnect budget\*\* por minuto (ex.: max 2 tentativas)  
\*   “New account quarantine”: limita qualidade e frequência até reputação

\*\*Drivers de custo\*\*

\*   egress é dominado pelo tempo em call e bitrate.

\* \* \*

E) Private Call (invite)  
\------------------------

\*\*Stage:\*\* CallUI \+ consent overlay

\*   CP: consent handshake \+ sessions \+ chat opcional  
\*   MP: WebRTC \`call\_private\_std\`

\*\*COST\\\_LOW caps\*\*

\*   480p→720p adaptativo, menos agressivo que Random  
\*   broadcast audio “duck” (se você mantiver Party no fundo)

\* \* \*

F) Meeting  
\----------

\*\*Stage:\*\* grid \+ chat room

\*   CP: room state/perms \+ moderação  
\*   MP: \`meeting\_adaptive\`

\*\*COST\\\_LOW caps\*\*

\*   cap por N (acima), active speaker sempre

\* \* \*

G) Live Public (Performer stage)  
\--------------------------------

\*\*Stage:\*\* player \+ chat público (separado do Party)

\*   CP: room metadata \+ chat \+ moderação reforçada  
\*   MP: \*\*evitar Stream no começo\*\* se você não quer custo variável; se usar, saiba que é “minutos entregues”.

\* \* \*

3\) Fórmulas de custo (Realtime) — pra você orçar sem chute  
\==========================================================

Conversão rápida  
\----------------

\*\*GB por hora ≈ 0,439 × Mbps\\\_recebidos\\\_por\\\_usuário\*\*

\#\#\# Party thumbs (exemplo COST\\\_LOW)

\*   12 tiles × 0,15 Mbps \= \*\*1,8 Mbps\*\*  
\*   GB/h por viewer ≈ 0,439 × 1,8 \= \*\*0,79 GB/h\*\*  
\*   Egress total por hora ≈ \`0,79 × viewers\_concorrentes\`

\#\#\# Random call (exemplo default 480p)

\*   Vídeo 0,9 Mbps \+ áudio 0,03 Mbps ≈ \*\*0,93 Mbps por usuário\*\*  
\*   GB/h por usuário ≈ 0,439 × 0,93 \= \*\*0,41 GB/h\*\*  
\*   Egress por call (2 usuários) ≈ \*\*0,82 GB/h\*\*  
\*   Por 10 min de call ≈ \*\*0,14 GB\*\* egress total

\> Depois do free tier, custo ≈ \`egress\_GB × $0,05\`.

Isso te dá um “painel mental” bem claro: \*\*Party custa por concorrência\*\*, Random custa por \*\*minutos em call\*\*.

\* \* \*

4\) “Cost switches” no Cartão do Dono (1 slider pra tudo)  
\========================================================

\*   \*\*COST\\\_LOW\*\* (default):  
    \*   thumbs\\\_low, tile limits, Random 480p  
\*   \*\*BALANCED\*\*:  
    \*   thumbs 240p/10fps, Random 540p default  
\*   \*\*QUALITY\\\_MAX\*\*:  
    \*   thumbs maiores \+ mais tiles (com teto), Random 720p default

Sem mudar código: só policy.

\* \* \*

5\) Implementação mínima (pra Random não virar inferno)  
\======================================================

Backends que precisam existir \*\*já no MVP com Random\*\*:

1\.  Auth/flags/quarantine  
2\.  Presence (Party)  
3\.  Chat (Party \+ privado)  
4\.  Session orchestrator (transition \+ endpoints)  
5\.  Matching service (queue/skip/cooldown)  
6\.  Moderation (report/block/ban)  
7\.  MediaProvider (Realtime SFU/TURN)  
8\.  Telemetry (mínimo: call\\\_minutes, skips, reports, egress estimado)

\* \* \*

Se você quiser, eu monto a \*\*matriz em formato “tabela de config”\*\* (tipo YAML/JSON) com esses perfis e limites (desktop/mobile) pra virar \*\*fonte da verdade\*\* do projeto — e aí o frontend só lê isso e aplica.

\#\# Prompt:  
ok

\#\# Response:  
Fechado ✅

Se você quiser transformar essa matriz em algo \*\*direto pro repo\*\*, eu consigo te entregar em 3 arquivos (bem DRY):

\*   \`media-profiles.yaml\` (thumb\\\_low / call\\\_random\\\_fast / meeting\\\_adaptive…)  
\*   \`mode-manifests.yaml\` (party/sniffers/random com caps por device)  
\*   \`cost-tiers.yaml\` (COST\\\_LOW/BALANCED/QUALITY\\\_MAX)

Aí o app e o backend só consomem config, sem hardcode.

\#\# Prompt:  
vai\!

\#\# Response:  
Thought for 27s

Massa — aqui vão os \*\*3 arquivos YAML\*\* prontos pra colar no repo. Eles foram desenhados pra você \*\*não hardcodar nada\*\*: o app (web) e o backend só \*\*leem config\*\*, e você troca o “slider” de custo/qualidade via \`cost-tiers.yaml\`.

\> \*\*Como usar (bem simples):\*\* carregue \`media-profiles.yaml\` \+ \`mode-manifests.yaml\`, depois aplique overrides do tier escolhido em \`cost-tiers.yaml\` (merge profundo).    
\> O resultado final vira a “fonte da verdade” para caps, layouts, limites e perfis de mídia.

\* \* \*

\`\`\`  
\# media-profiles.yaml  
version: 1  
updated\_at: "2026-03-01"  
owner: "voulezvous"  
purpose: "Single source of truth for media constraints (WebRTC/HLS) used by all modes."

\# Notes:  
\# \- All bitrates are caps (upper bounds). Actual bitrate is adaptive.  
\# \- Audio defaults to Opus when enabled.  
\# \- These profiles are referenced by mode manifests and can be overridden by cost tiers.

profiles:  
  \# Presence thumbs (Party/Sniffers): cheap by default  
  thumb\_low:  
    kind: webrtc  
    intent\_label: presence\_thumb  
    webrtc:  
      video:  
        max\_width: 320  
        max\_height: 180  
        fps\_max: 8  
        bitrate\_kbps\_max: 180  
        codec\_preference: \["VP8", "H264"\]  
        simulcast:  
          enabled: false  
      audio:  
        enabled: false  
      connection:  
        \# Prefer fast connect; keep retries limited to avoid cost loops.  
        reconnect\_budget:  
          attempts\_per\_minute: 2  
          backoff\_ms: \[0, 500, 1500\]  
        ice:  
          transport\_policy: "all"

  \# Random call: optimized for fast connect \+ reasonable cost  
  call\_random\_fast:  
    kind: webrtc  
    intent\_label: random\_call  
    webrtc:  
      video:  
        codec\_preference: \["VP8", "H264"\]  
        simulcast:  
          enabled: true  
          layers:  
            \- name: sd360  
              max\_width: 640  
              max\_height: 360  
              fps\_max: 24  
              bitrate\_kbps\_max: 700  
            \- name: sd480  
              max\_width: 854  
              max\_height: 480  
              fps\_max: 24  
              bitrate\_kbps\_max: 1000  
            \- name: hd720  
              max\_width: 1280  
              max\_height: 720  
              fps\_max: 30  
              bitrate\_kbps\_max: 2200  
        adaptation:  
          \# Upgrade only under good conditions (keeps costs predictable)  
          upgrade\_rules:  
            require\_wifi: true  
            max\_rtt\_ms: 200  
            max\_jitter\_ms: 35  
            max\_loss\_percent: 2  
            min\_bitrate\_headroom\_kbps: 600  
          downgrade\_rules:  
            max\_rtt\_ms: 350  
            max\_jitter\_ms: 60  
            max\_loss\_percent: 5  
      audio:  
        enabled: true  
        codec: "opus"  
        bitrate\_kbps\_max: 32  
      connection:  
        reconnect\_budget:  
          attempts\_per\_minute: 2  
          backoff\_ms: \[0, 500, 1500\]  
        ice:  
          transport\_policy: "all"

  \# Private call: slightly less aggressive than random (quality bias)  
  call\_private\_std:  
    kind: webrtc  
    intent\_label: private\_call  
    webrtc:  
      video:  
        codec\_preference: \["VP8", "H264"\]  
        simulcast:  
          enabled: true  
          layers:  
            \- name: sd480  
              max\_width: 854  
              max\_height: 480  
              fps\_max: 24  
              bitrate\_kbps\_max: 1100  
            \- name: hd720  
              max\_width: 1280  
              max\_height: 720  
              fps\_max: 30  
              bitrate\_kbps\_max: 2400  
        adaptation:  
          upgrade\_rules:  
            require\_wifi: false  
            max\_rtt\_ms: 240  
            max\_jitter\_ms: 45  
            max\_loss\_percent: 3  
            min\_bitrate\_headroom\_kbps: 500  
          downgrade\_rules:  
            max\_rtt\_ms: 380  
            max\_jitter\_ms: 70  
            max\_loss\_percent: 6  
      audio:  
        enabled: true  
        codec: "opus"  
        bitrate\_kbps\_max: 40  
      connection:  
        reconnect\_budget:  
          attempts\_per\_minute: 3  
          backoff\_ms: \[0, 400, 1200\]  
        ice:  
          transport\_policy: "all"

  \# Meeting: adapt by participant count to avoid explosions  
  meeting\_adaptive:  
    kind: webrtc  
    intent\_label: meeting\_room  
    webrtc:  
      video:  
        codec\_preference: \["VP8", "H264"\]  
        layout\_strategy: "active\_speaker\_priority"  
        caps\_by\_participants:  
          "1-4":  
            max\_width: 1280  
            max\_height: 720  
            fps\_max: 24  
            bitrate\_kbps\_max: 1800  
          "5-8":  
            max\_width: 854  
            max\_height: 480  
            fps\_max: 20  
            bitrate\_kbps\_max: 1200  
          "9-99":  
            max\_width: 640  
            max\_height: 360  
            fps\_max: 15  
            bitrate\_kbps\_max: 900  
      audio:  
        enabled: true  
        codec: "opus"  
        bitrate\_kbps\_max: 40  
      connection:  
        reconnect\_budget:  
          attempts\_per\_minute: 2  
          backoff\_ms: \[0, 700, 2000\]  
        ice:  
          transport\_policy: "all"

  \# Broadcast (VVTV) is LAB-first. This profile describes player expectations only.  
  broadcast\_hls\_lab:  
    kind: hls  
    intent\_label: broadcast  
    hls:  
      \# Ladder is controlled by LAB encoder. Keep it small initially.  
      ladder:  
        \- name: low  
          max\_width: 854  
          max\_height: 480  
          target\_video\_kbps: 900  
          target\_audio\_kbps: 96  
        \- name: mid  
          max\_width: 1280  
          max\_height: 720  
          target\_video\_kbps: 1800  
          target\_audio\_kbps: 128  
      player:  
        live\_latency\_target\_s: 8  
        startup\_buffer\_s: 3  
        retry\_policy:  
          max\_retries: 8  
          backoff\_ms: \[0, 500, 1500, 3000, 5000, 8000\]  
\`\`\`

\* \* \*

\`\`\`  
\# mode-manifests.yaml  
version: 1  
updated\_at: "2026-03-01"  
purpose: "Defines UI composition, routes, required capabilities, and which media profiles each mode uses."

\# Shell is always the same. Modes only change composition/config.  
shell:  
  topbar:  
    style: minimal  
    show\_mode\_pill: true  
    show\_connection\_badge: true  
  overlays\_global:  
    \- id: toasts  
    \- id: transition\_layer  
    \- id: moderation\_sheet

routing:  
  \# Rule: login always lands in Party unless a strong deep link exists.  
  post\_login\_default\_route: "/party"  
  deep\_link\_priority\_routes:  
    \- "/call/:token"  
    \- "/meeting/:room\_id"  
    \- "/live/:room\_id"

modes:  
  broadcast:  
    id: broadcast  
    route: "/"  
    label: "VVTV"  
    stage:  
      component: main\_player  
      media\_profile: broadcast\_hls\_lab  
    rail: null  
    overlays:  
      \- id: login\_modal  
    capabilities:  
      \- broadcast\_view  
    policies:  
      allow\_anonymous: true  
      require\_login\_to\_interact: true  
      login\_redirect\_route: "/party"

  party:  
    id: party  
    route: "/party"  
    label: "Party"  
    stage:  
      component: main\_player  
      media\_profile: broadcast\_hls\_lab  
      audio\_layer:  
        enabled: true  
        component: audio\_player  
        \# music is optional; controlled by policy/card.  
        mode: "party\_music"  
    rail:  
      component: presence\_rail  
      data\_source: "presence:party\_presence"  
      tile\_component: video\_tile  
      tile\_media\_profile: thumb\_low  
    overlays:  
      \- id: chat\_overlay  
        scope: private\_1to1  
        allow\_parallel\_during\_party: true  
      \- id: consent\_overlay  
      \- id: moderation\_sheet  
    capabilities:  
      \- auth\_required  
      \- presence  
      \- chat\_private  
      \- sessions  
      \- moderation\_basic  
    ui\_limits:  
      tiles\_render:  
        desktop: 24  
        mobile: 12  
      tiles\_active:  
        desktop: 12  
        mobile: 6  
      connect\_strategy: "viewport\_only"   \# only connect WebRTC if visible  
      profile\_card:  
        enabled: true  
        actions: \["chat", "invite\_call", "report", "block"\]  
    transitions:  
      default\_handover: "crossfade\_250ms"  
      on\_call\_start:  
        broadcast\_audio\_duck: true  
        broadcast\_pip: false  
      on\_call\_end:  
        return\_route: "/party"  
    actions:  
      invite\_to\_call:  
        session\_mode: private\_call  
        call\_media\_profile: call\_private\_std  
        consent\_required: true  
        timeout\_s: 20

  sniffers:  
    id: sniffers  
    route: "/sniffers"  
    label: "Sniffers"  
    stage:  
      component: grid\_discovery  
      data\_source: "discovery:grid\_presence"  
      tile\_component: video\_tile  
      tile\_media\_profile: thumb\_low  
    rail:  
      component: filters\_panel  
      filters:  
        \- id: online\_only  
        \- id: tags  
        \- id: language  
        \- id: safety\_level  
    overlays:  
      \- id: chat\_overlay  
        scope: private\_1to1  
      \- id: moderation\_sheet  
    capabilities:  
      \- auth\_required  
      \- presence  
      \- discovery  
      \- chat\_private  
      \- sessions  
      \- moderation\_basic  
    ui\_limits:  
      tiles\_render:  
        desktop: 32  
        mobile: 16  
      tiles\_active:  
        desktop: 12  
        mobile: 6  
      connect\_strategy: "viewport\_only"  
      profile\_card:  
        enabled: true  
        actions: \["chat", "invite\_call", "report", "block"\]  
    actions:  
      invite\_to\_call:  
        session\_mode: private\_call  
        call\_media\_profile: call\_private\_std  
        consent\_required: true  
        timeout\_s: 20

  random:  
    id: random  
    route: "/random"  
    label: "Random"  
    stage:  
      component: call\_ui  
      call\_media\_profile: call\_random\_fast  
    rail:  
      component: queue\_panel  
      enabled: true  
      options:  
        \- id: language\_pref  
        \- id: tags\_pref  
    overlays:  
      \- id: match\_overlay  
      \- id: moderation\_sheet  
    capabilities:  
      \- auth\_required  
      \- matching\_random  
      \- sessions  
      \- moderation\_strong  
    controls:  
      fixed\_actions: \["skip", "mute\_cam", "mute\_mic", "report", "block"\]  
    matching:  
      queue: "random"  
      \# Anti-abuse \+ cost control:  
      min\_time\_before\_skip\_ms: 2000  
      cooldown\_reset\_after\_s: 600  
      \# index \= skip\_count; value \= cooldown seconds  
      cooldown\_schedule\_s: \[0, 0, 0, 3, 5, 8, 15, 30, 60\]  
      reconnect\_budget:  
        attempts\_per\_minute: 2  
        backoff\_ms: \[0, 500, 1500\]  
      quarantine:  
        enabled: true  
        rules:  
          new\_account\_days: 3  
          max\_matches\_per\_hour: 20  
          force\_low\_quality\_profile: true  
    transitions:  
      default\_handover: "cut\_120ms"  
      on\_end:  
        return\_route: "/random"

  invite\_call:  
    id: invite\_call  
    route: "/call/:token"  
    label: "Invite"  
    stage:  
      component: invite\_entry  
    overlays:  
      \- id: consent\_overlay  
      \- id: moderation\_sheet  
    capabilities:  
      \- sessions  
      \- moderation\_basic  
    policies:  
      require\_login: true  
      on\_accept:  
        session\_mode: private\_call  
        call\_media\_profile: call\_private\_std  
      on\_reject:  
        logged\_in\_route: "/party"  
        anonymous\_route: "/"

  meeting:  
    id: meeting  
    route: "/meeting/:room\_id"  
    label: "Meeting"  
    stage:  
      component: meeting\_grid  
      call\_media\_profile: meeting\_adaptive  
    rail:  
      component: room\_info\_panel  
      enabled: true  
    overlays:  
      \- id: chat\_overlay  
        scope: room  
      \- id: moderation\_sheet  
    capabilities:  
      \- auth\_required  
      \- sessions  
      \- chat\_room  
      \- moderation\_basic  
    policies:  
      participant\_limit\_default: 12

  live\_public:  
    id: live\_public  
    route: "/live/:room\_id"  
    label: "Live"  
    stage:  
      component: performer\_stage\_player  
      \# Implementation note: stage delivery can be HLS/Stream later; keep separate from Party.  
    rail:  
      component: room\_chat  
      scope: performer\_room  
      slow\_mode\_default\_s: 3  
    overlays:  
      \- id: moderation\_sheet  
    capabilities:  
      \- auth\_required  
      \- chat\_room  
      \- moderation\_strong  
    policies:  
      separate\_ecosystem\_from\_party: true

  mirror:  
    id: mirror  
    route: "/mirror"  
    label: "Mirror"  
    stage:  
      component: mirror\_view  
    overlays:  
      \- id: open\_to\_sheet  
    capabilities:  
      \- auth\_required  
      \- sessions  
    actions:  
      open\_to:  
        creates\_new\_session: true  
        default\_session\_mode: private\_call  
        call\_media\_profile: call\_private\_std  
\`\`\`

\* \* \*

\`\`\`  
\# cost-tiers.yaml  
version: 1  
updated\_at: "2026-03-01"  
purpose: "Budget knobs. Apply as deep-merge overrides over media-profiles \+ mode-manifests."

\# How to apply:  
\# 1\) Load base configs (media-profiles.yaml \+ mode-manifests.yaml)  
\# 2\) Choose tier (COST\_LOW/BALANCED/QUALITY\_MAX)  
\# 3\) Deep-merge: base \<- tier.overrides  
\# 4\) (Optional) enforce hard kill-switches at runtime

tiers:  
  COST\_LOW:  
    description: "LAB-first. Cheapest stable defaults. Strict limits on tiles and call quality upgrades."  
    overrides:  
      \# Mode UI limits  
      modes:  
        party:  
          ui\_limits:  
            tiles\_active: { desktop: 12, mobile: 6 }  
            tiles\_render: { desktop: 24, mobile: 12 }  
        sniffers:  
          ui\_limits:  
            tiles\_active: { desktop: 12, mobile: 6 }  
            tiles\_render: { desktop: 32, mobile: 16 }  
        random:  
          matching:  
            reconnect\_budget: { attempts\_per\_minute: 2, backoff\_ms: \[0, 500, 1500\] }

      \# Media profile tightening  
      profiles:  
        thumb\_low:  
          webrtc:  
            video:  
              fps\_max: 8  
              bitrate\_kbps\_max: 160  
        call\_random\_fast:  
          webrtc:  
            video:  
              simulcast:  
                layers:  
                  \- name: sd360  
                    bitrate\_kbps\_max: 650  
                  \- name: sd480  
                    bitrate\_kbps\_max: 950  
                  \- name: hd720  
                    bitrate\_kbps\_max: 2000  
              adaptation:  
                upgrade\_rules:  
                  require\_wifi: true  
        meeting\_adaptive:  
          webrtc:  
            video:  
              caps\_by\_participants:  
                "1-4": { bitrate\_kbps\_max: 1600, fps\_max: 22 }  
                "5-8": { bitrate\_kbps\_max: 1100, fps\_max: 18 }  
                "9-99": { bitrate\_kbps\_max: 850, fps\_max: 15 }

    kill\_switches:  
      \# Hard limits regardless of UI (protects you from UI bugs / abuse)  
      max\_active\_presence\_tiles\_per\_user: 12  
      max\_active\_presence\_tiles\_per\_user\_mobile: 6  
      max\_random\_matches\_per\_user\_per\_hour: 60  
      max\_random\_skips\_per\_minute: 12  
      \# If you implement budget monitoring, enforce these:  
      max\_estimated\_egress\_gb\_per\_hour\_total: 500

  BALANCED:  
    description: "Middle ground. Better thumbs \+ a bit more headroom on calls."  
    overrides:  
      modes:  
        party:  
          ui\_limits:  
            tiles\_active: { desktop: 16, mobile: 8 }  
            tiles\_render: { desktop: 32, mobile: 16 }  
        sniffers:  
          ui\_limits:  
            tiles\_active: { desktop: 16, mobile: 8 }  
      profiles:  
        thumb\_low:  
          webrtc:  
            video:  
              max\_width: 426  
              max\_height: 240  
              fps\_max: 10  
              bitrate\_kbps\_max: 260  
        call\_random\_fast:  
          webrtc:  
            video:  
              simulcast:  
                layers:  
                  \- name: sd360  
                    bitrate\_kbps\_max: 800  
                  \- name: sd480  
                    bitrate\_kbps\_max: 1200  
                  \- name: hd720  
                    bitrate\_kbps\_max: 2600  
              adaptation:  
                upgrade\_rules:  
                  require\_wifi: false  
                  max\_rtt\_ms: 220  
                  max\_loss\_percent: 3  
    kill\_switches:  
      max\_active\_presence\_tiles\_per\_user: 16  
      max\_active\_presence\_tiles\_per\_user\_mobile: 8  
      max\_random\_matches\_per\_user\_per\_hour: 90  
      max\_random\_skips\_per\_minute: 15  
      max\_estimated\_egress\_gb\_per\_hour\_total: 1200

  QUALITY\_MAX:  
    description: "Best experience. Use only if you accept higher variable cost."  
    overrides:  
      modes:  
        party:  
          ui\_limits:  
            tiles\_active: { desktop: 20, mobile: 10 }  
            tiles\_render: { desktop: 40, mobile: 20 }  
        sniffers:  
          ui\_limits:  
            tiles\_active: { desktop: 20, mobile: 10 }  
      profiles:  
        thumb\_low:  
          webrtc:  
            video:  
              max\_width: 640  
              max\_height: 360  
              fps\_max: 12  
              bitrate\_kbps\_max: 420  
        call\_random\_fast:  
          webrtc:  
            video:  
              simulcast:  
                layers:  
                  \- name: sd360  
                    bitrate\_kbps\_max: 900  
                  \- name: sd480  
                    bitrate\_kbps\_max: 1500  
                  \- name: hd720  
                    bitrate\_kbps\_max: 3200  
              adaptation:  
                upgrade\_rules:  
                  require\_wifi: false  
                  max\_rtt\_ms: 260  
                  max\_loss\_percent: 4  
    kill\_switches:  
      max\_active\_presence\_tiles\_per\_user: 20  
      max\_active\_presence\_tiles\_per\_user\_mobile: 10  
      max\_random\_matches\_per\_user\_per\_hour: 120  
      max\_random\_skips\_per\_minute: 20  
      max\_estimated\_egress\_gb\_per\_hour\_total: 2500  
\`\`\`

\* \* \*

Se você quiser ir um passo além (bem útil), eu te mando também:

\*   um \`copy.json\` (microcopy padronizada por keys),  
\*   e um \`telemetry-events.yaml\` (eventos mínimos por modo: viewer→login→party→chat→call, skips, reports, QoS).

\#\# Prompt:  
sim

\#\# Response:  
Perfeito — aqui vão os \*\*dois arquivos extras\*\*: \`copy.json\` (microcopy centralizada) e \`telemetry-events.yaml\` (eventos mínimos \+ payloads). A ideia é \*\*zero string solta\*\* e \*\*telemetria padronizada\*\* pra você medir funil/custo/abuso sem inventar a cada modo. 😄📈

\* \* \*

\`\`\`  
{  
  "version": 1,  
  "updated\_at": "2026-03-01",  
  "locale": "pt-BR",  
  "app": "voulezvous",  
  "strings": {  
    "topbar.enter": "Entrar",  
    "topbar.back\_to\_party": "Voltar pro Party",  
    "topbar.mode.broadcast": "VVTV",  
    "topbar.mode.party": "Party",  
    "topbar.mode.sniffers": "Sniffers",  
    "topbar.mode.random": "Random",  
    "topbar.mode.live": "Live",  
    "topbar.mode.mirror": "Mirror",  
    "topbar.connection.good": "Conectado",  
    "topbar.connection.bad": "Conexão instável…",

    "broadcast.now\_playing": "Agora",  
    "broadcast.retry": "Tentar novamente",  
    "broadcast.error": "O canal deu uma falhada. Tentando reconectar…",  
    "broadcast.login\_hint": "Entre para ir pro Party.",

    "party.header": "Agora.",  
    "party.online\_now": "online agora",  
    "party.my\_cam.on": "Minha cam: ligada",  
    "party.my\_cam.off": "Minha cam: desligada",  
    "party.open\_chat": "Chat",  
    "party.invite\_call": "Convidar pra call",  
    "party.report": "Reportar",  
    "party.block": "Bloquear",

    "sniffers.empty": "Ninguém online com esses filtros.",  
    "sniffers.clear\_filters": "Limpar filtros",  
    "sniffers.filters.title": "Filtros",  
    "sniffers.filters.online\_only": "Só online",  
    "sniffers.filters.tags": "Tags",  
    "sniffers.filters.language": "Idioma",  
    "sniffers.filters.safety": "Nível de segurança",

    "chat.title": "Chat",  
    "chat.placeholder": "Escreve aqui…",  
    "chat.send": "Enviar",  
    "chat.report": "Reportar",  
    "chat.block": "Bloquear",  
    "chat.invite\_call": "Convidar pra call",  
    "chat.invite\_sent": "Convite enviado.",  
    "chat.invite\_accepted": "Convite aceito ✅",  
    "chat.invite\_rejected": "Convite recusado.",

    "consent.title": "Convite pra call",  
    "consent.accept": "Aceitar",  
    "consent.reject": "Recusar",  
    "consent.expired": "Esse convite expirou.",  
    "consent.back\_party": "Voltar ao Party",  
    "consent.back\_tv": "Voltar pra TV",

    "call.connecting": "Chamando…",  
    "call.connected": "Conectado ✅",  
    "call.ended": "Chamada encerrada.",  
    "call.mute\_cam": "Cam",  
    "call.mute\_mic": "Mic",  
    "call.hangup": "Sair",

    "random.title": "Random",  
    "random.start": "Começar",  
    "random.searching": "Procurando alguém…",  
    "random.connecting": "Conectando…",  
    "random.skip": "Pular",  
    "random.cooldown.title": "Pausa rápida pra segurança",  
    "random.cooldown.body": "Você pode tentar de novo em {time}.",  
    "random.report": "Reportar",  
    "random.block": "Bloquear",

    "meeting.title": "Meeting",  
    "meeting.participants": "Participantes",  
    "meeting.leave": "Sair",

    "live.title": "Live",  
    "live.chat.title": "Chat da sala",  
    "live.slowmode": "Devagar 🙂",

    "mirror.title": "Mirror",  
    "mirror.open\_to": "Abrir para…",

    "toast.login.success": "Logado ✅ Bem-vindo ao Party.",  
    "toast.login.cancel": "Login cancelado.",  
    "toast.login.fail": "Não deu certo. Tenta de novo.",  
    "toast.presence.cam\_on": "Sua cam está ligada.",  
    "toast.presence.cam\_off": "Sua cam está desligada.",  
    "toast.net.unstable": "Conexão instável… ajustando.",  
    "toast.moderation.report\_received": "Report recebido. Obrigado.",  
    "toast.moderation.blocked": "Usuário bloqueado.",  
    "toast.moderation.unblocked": "Bloqueio removido."  
  }  
}  
\`\`\`

\* \* \*

\`\`\`  
\# telemetry-events.yaml  
version: 1  
updated\_at: "2026-03-01"  
purpose: "Minimum analytics \+ safety telemetry for VVTV \+ Party \+ Random. Single schema for all modes."

\# Guidelines:  
\# \- All events should include: ts\_ms, user\_id (or anonymous\_id), session\_id (if any), mode\_id, route, device\_type.  
\# \- Avoid sensitive content. Never log message bodies, video content, or explicit identifiers beyond internal IDs.  
\# \- Use sampling for high-frequency QoS pings.

common\_fields:  
  required:  
    \- ts\_ms  
    \- mode\_id  
    \- route  
    \- device\_type   \# desktop|mobile|tablet  
  optional:  
    \- user\_id  
    \- anonymous\_id  
    \- presence\_id  
    \- session\_id  
    \- thread\_id  
    \- room\_id  
    \- cohort\_id  
    \- cost\_tier     \# COST\_LOW|BALANCED|QUALITY\_MAX  
    \- build\_version

events:

  \# \--- Funnel / navigation \---  
  app\_open:  
    description: "User opens the app (any route)."  
    fields:  
      required: \[ts\_ms, route, device\_type\]  
      optional: \[user\_id, anonymous\_id, build\_version\]

  view\_broadcast:  
    description: "Broadcast viewer starts or resumes playback."  
    fields:  
      required: \[ts\_ms, mode\_id, route, device\_type\]  
      optional: \[anonymous\_id, user\_id, stream\_health\]

  click\_login:  
    description: "User clicks login entry point."  
    fields:  
      required: \[ts\_ms, route, device\_type\]  
      optional: \[anonymous\_id\]

  auth\_success:  
    description: "Login succeeded."  
    fields:  
      required: \[ts\_ms, device\_type\]  
      optional: \[user\_id, from\_route, to\_route\]

  enter\_mode:  
    description: "User enters a mode (Party/Sniffers/Random/etc.)."  
    fields:  
      required: \[ts\_ms, mode\_id, route, device\_type\]  
      optional: \[user\_id, from\_mode\_id, cost\_tier\]

  \# \--- Presence \---  
  presence\_set:  
    description: "User updates presence state (cam on/off, mode)."  
    fields:  
      required: \[ts\_ms, mode\_id, device\_type\]  
      optional: \[user\_id, presence\_mode, is\_camera\_on\]

  presence\_tile\_connect:  
    description: "Client starts receiving a presence tile stream (WebRTC)."  
    fields:  
      required: \[ts\_ms, mode\_id, device\_type\]  
      optional: \[user\_id, presence\_id, tile\_index, media\_profile, reason\] \# viewport|tap|auto

  presence\_tile\_disconnect:  
    description: "Client stops receiving a presence tile stream."  
    fields:  
      required: \[ts\_ms, mode\_id, device\_type\]  
      optional: \[user\_id, presence\_id, duration\_ms, reason\] \# out\_of\_view|close|budget\_guard

  \# \--- Chat \---  
  chat\_thread\_open:  
    description: "User opens a chat thread overlay."  
    fields:  
      required: \[ts\_ms, mode\_id, device\_type\]  
      optional: \[user\_id, thread\_id, created\_from\] \# party\_presence|sniffers\_grid|invite

  chat\_message\_send:  
    description: "User sends a message (no content logged)."  
    fields:  
      required: \[ts\_ms, mode\_id, device\_type\]  
      optional: \[user\_id, thread\_id, msg\_len, attachments\_count\]

  chat\_rate\_limited:  
    description: "Chat message blocked due to rate limit."  
    fields:  
      required: \[ts\_ms, mode\_id, device\_type\]  
      optional: \[user\_id, thread\_id, limit\_bucket\]

  \# \--- Calls / Sessions \---  
  session\_create:  
    description: "Backend creates a session."  
    fields:  
      required: \[ts\_ms, mode\_id\]  
      optional: \[user\_id, session\_id, session\_mode, parent\_session\_id, intent\_label, media\_profile\]

  session\_transition:  
    description: "Mode change creates a new session."  
    fields:  
      required: \[ts\_ms, mode\_id\]  
      optional: \[user\_id, from\_session\_id, to\_session\_id, to\_mode, handover\_hint\]

  call\_invite\_sent:  
    description: "User sends a call invite."  
    fields:  
      required: \[ts\_ms, mode\_id, device\_type\]  
      optional: \[user\_id, thread\_id, session\_id, media\_profile\]

  call\_invite\_received:  
    description: "User receives a call invite."  
    fields:  
      required: \[ts\_ms, mode\_id, device\_type\]  
      optional: \[user\_id, session\_id, from\_user\_id\_hash\]

  consent\_accept:  
    description: "User accepts consent overlay."  
    fields:  
      required: \[ts\_ms, mode\_id, device\_type\]  
      optional: \[user\_id, session\_id\]

  consent\_reject:  
    description: "User rejects consent overlay."  
    fields:  
      required: \[ts\_ms, mode\_id, device\_type\]  
      optional: \[user\_id, session\_id, reason\] \# no\_reason|safety|not\_now

  webrtc\_connect\_start:  
    description: "Client begins WebRTC connection."  
    fields:  
      required: \[ts\_ms, mode\_id, device\_type\]  
      optional: \[user\_id, session\_id, media\_profile, network\_type\] \# wifi|cell|unknown

  webrtc\_connect\_success:  
    description: "WebRTC connected successfully."  
    fields:  
      required: \[ts\_ms, mode\_id, device\_type\]  
      optional: \[user\_id, session\_id, connect\_time\_ms, selected\_layer\]

  webrtc\_connect\_fail:  
    description: "WebRTC connection failed."  
    fields:  
      required: \[ts\_ms, mode\_id, device\_type\]  
      optional: \[user\_id, session\_id, error\_code, stage\] \# ice|dtls|sfu|turn

  call\_end:  
    description: "Call ended."  
    fields:  
      required: \[ts\_ms, mode\_id, device\_type\]  
      optional: \[user\_id, session\_id, duration\_ms, end\_reason\] \# hangup|remote|timeout|moderation

  \# \--- Random-specific \---  
  random\_join\_queue:  
    description: "User joins random queue."  
    fields:  
      required: \[ts\_ms, mode\_id, device\_type\]  
      optional: \[user\_id, prefs\_hash, quarantine\]

  random\_match\_found:  
    description: "Match found event."  
    fields:  
      required: \[ts\_ms, mode\_id\]  
      optional: \[user\_id, session\_id, cohort\_id, match\_latency\_ms\]

  random\_skip:  
    description: "User presses skip."  
    fields:  
      required: \[ts\_ms, mode\_id, device\_type\]  
      optional: \[user\_id, session\_id, time\_in\_call\_ms, skip\_count\_in\_window\]

  random\_cooldown\_start:  
    description: "Cooldown enforced."  
    fields:  
      required: \[ts\_ms, mode\_id, device\_type\]  
      optional: \[user\_id, cooldown\_s, reason\] \# skip\_spam|new\_account|reports

  \# \--- Moderation / Safety \---  
  report\_submit:  
    description: "User submits a report."  
    fields:  
      required: \[ts\_ms, mode\_id, device\_type\]  
      optional: \[user\_id, target\_user\_id, session\_id, thread\_id, reason\_code\]

  block\_user:  
    description: "User blocks another user."  
    fields:  
      required: \[ts\_ms, mode\_id, device\_type\]  
      optional: \[user\_id, target\_user\_id, context\] \# party|chat|random|live

  ban\_enforced:  
    description: "Backend enforces a ban/quarantine action."  
    fields:  
      required: \[ts\_ms, mode\_id\]  
      optional: \[user\_id, action\] \# ban|shadow|quarantine|cooldown, duration\_s

  \# \--- QoS sampling (high-frequency; sample\!) \---  
  qos\_sample:  
    description: "Periodic QoS sample (sampled)."  
    fields:  
      required: \[ts\_ms, mode\_id, device\_type\]  
      optional:  
        \- user\_id  
        \- session\_id  
        \- media\_profile  
        \- rtt\_ms  
        \- jitter\_ms  
        \- loss\_percent  
        \- downlink\_kbps\_est  
        \- selected\_layer  
        \- sample\_rate

privacy:  
  do\_not\_log:  
    \- chat\_message\_body  
    \- video\_frames  
    \- explicit\_titles\_or\_tags\_from\_content  
    \- precise\_geolocation  
  hashing:  
    from\_user\_id\_hash: "hash(user\_id) with rotating salt"

sampling:  
  qos\_sample:  
    default\_sample\_rate: 0.1  \# 10%  
    increase\_to: 1.0          \# on errors or during incidents  
\`\`\`

\* \* \*

Se você quiser deixar isso ainda mais “plugável”, eu também posso adicionar:

\*   \`schema.md\` com “como fazer deep-merge” e regras de validação (ex.: campos obrigatórios, ranges de bitrate/fps),  
\*   e um \`runtime-guards.yaml\` separado (kill-switches por tier \+ limites por IP/user).

\#\# Prompt:  
sim

\#\# Response:  
Boa 😄✅ Aqui vão os dois extras: \*\*\`schema.md\`\*\* (regras de merge/validação \+ invariantes) e \*\*\`runtime-guards.yaml\`\*\* (guardrails operacionais independentes do tier). A ideia é: \*\*config define UX/caps\*\*, mas \*\*guards protegem\*\* mesmo que alguém quebre o frontend ou tente abusar.

\* \* \*

\`\`\`  
\# schema.md  
Version: 1  
Updated: 2026-03-01

\#\# Objetivo  
Este documento define:  
1\) Como combinar configs (\`media-profiles.yaml\`, \`mode-manifests.yaml\`, \`cost-tiers.yaml\`, \`runtime-guards.yaml\`);  
2\) Regras de validação (invariantes) para impedir valores perigosos;  
3\) “Fonte da verdade” e responsabilidade de cada arquivo.

\---

\#\# 1\) Fonte da verdade (o que manda em quê)

\#\#\# media-profiles.yaml  
\- Define \*\*restrições de mídia\*\* (WebRTC/HLS): resolução, fps, bitrate, simulcast, regras de upgrade/downgrade, reconnect budgets.  
\- Deve ser referenciado por \`mode-manifests.yaml\` via \`media\_profile\`.

\#\#\# mode-manifests.yaml  
\- Define \*\*composição de UI\*\*, rotas, capabilities, ações, limites por device (tiles/cooldowns), e qual perfil de mídia é usado em cada contexto.

\#\#\# cost-tiers.yaml  
\- Define \*\*overrides\*\* (deep merge) sobre os dois arquivos acima.  
\- Também define \*\*kill-switches\*\* do tier (limites globais para proteção do orçamento).

\#\#\# runtime-guards.yaml  
\- Define \*\*guardrails operacionais\*\* (rate limits, budgets, quotas, circuit breakers).  
\- Deve ser aplicado \*\*sempre\*\*, independentemente do tier (pode ter escalas por tier, mas nunca ser desligado).

\---

\#\# 2\) Algoritmo de merge (deep-merge canônico)

\#\#\# Inputs  
\- Base:  
  \- \`media-profiles.yaml\`  
  \- \`mode-manifests.yaml\`  
\- Tier:  
  \- \`cost-tiers.yaml\` → \`tiers\[SELECTED\].overrides\`  
  \- \`tiers\[SELECTED\].kill\_switches\`  
\- Guards:  
  \- \`runtime-guards.yaml\`

\#\#\# Passos  
1\) Carregar Base \= {profiles, modes, routing, shell...}  
2\) Aplicar \`tier.overrides\` via \*\*deep merge\*\*:  
   \- Map/obj: merge recursivo por chave  
   \- Lista: regra padrão é \*\*substituir lista inteira\*\* (evita merge ambíguo)  
   \- Exceção: listas de \`simulcast.layers\` podem ser mescladas por \`name\` (recomendado)  
3\) Validar invariantes (Seção 3). Se falhar: \*\*abort startup\*\* ou fallback para \`COST\_LOW\`.  
4\) Aplicar \`tier.kill\_switches\` e \`runtime-guards.yaml\` como “enforcement layer”:  
   \- Guards podem reduzir limites (min), mas nunca aumentar acima do tier.  
5\) Expor config final ao runtime:  
   \- Frontend consome: UI composition \+ ui\_limits \+ copy keys  
   \- Backend consome: profiles \+ matching policies \+ kill switches \+ guards

\---

\#\# 3\) Invariantes (validação obrigatória)

\#\#\# 3.1 Presença (Party/Sniffers)  
\- \`thumb\` deve ter \`audio.enabled \= false\` (sempre).  
\- \`tiles\_active.desktop \<= 20\` (hard max)  
\- \`tiles\_active.mobile \<= 10\` (hard max)  
\- \`connect\_strategy\` deve ser \`viewport\_only\` ou \`on\_demand\` (nunca \`always\_on\`).

\#\#\# 3.2 Random  
\- \`min\_time\_before\_skip\_ms \>= 1000\`  
\- \`cooldown\_schedule\_s\` deve ser monotônica não-decrescente após o índice 3\.  
\- \`reconnect\_budget.attempts\_per\_minute \<= 4\` (hard max)  
\- \`quarantine.enabled\` deve ser \`true\` no MVP com Random.

\#\#\# 3.3 WebRTC perfis  
\- \`bitrate\_kbps\_max\` por layer deve estar em intervalo seguro:  
  \- thumbs: 80–500 kbps  
  \- calls: 300–3500 kbps  
  \- meeting: 300–2500 kbps (dependendo de N)  
\- \`fps\_max\` não deve exceder:  
  \- thumbs: 15  
  \- calls: 30  
  \- meeting: 24  
\- \`simulcast.enabled=true\` para calls/meeting (recomendado), mas opcional se provider não suportar.

\#\#\# 3.4 Broadcast (HLS)  
\- Ladder deve ter no máximo 4 variantes no MVP.  
\- \`startup\_buffer\_s\` entre 2 e 6 segundos.  
\- \`retry\_policy.max\_retries \>= 5\`.

\#\#\# 3.5 Separação de ecossistemas  
\- \`live\_public.policies.separate\_ecosystem\_from\_party\` deve ser \`true\`.  
\- \`party\` não pode importar \`room\_chat\` público (somente \`chat\_overlay private\_1to1\`).

\---

\#\# 4\) Fallback seguro  
Se qualquer validação falhar em produção:  
\- Fallback automático para \`COST\_LOW\`  
\- Logar \`config\_validation\_failed\` (telemetry)  
\- Expor alerta no \`/status\`

\---

\#\# 5\) Ranges e defaults recomendados (MVP)  
\- Party thumbs: 180p, 6–8 fps, 120–180 kbps  
\- Random default: 480p, 20–24 fps, 700–1000 kbps, upgrade Wi-Fi only  
\- Meeting: cap por N \+ active speaker

\---

\#\# 6\) Responsabilidade do enforcement  
\- \*\*Frontend\*\*: respeitar ui\_limits, viewport gating, degrade UI  
\- \*\*Backend\*\*: aplicar guards sempre (rate limits, quotas, bans, cooldowns) e ignorar cliente malicioso  
\- \*\*MediaProvider\*\*: aplicar caps no servidor quando possível (SFU rules)  
\`\`\`

\* \* \*

\`\`\`  
\# runtime-guards.yaml  
version: 1  
updated\_at: "2026-03-01"  
purpose: "Operational guardrails enforced server-side to prevent abuse, runaway costs, and denial-of-wallet."

\# Enforcement priorities:  
\# 1\) Safety \> 2\) Budget protection \> 3\) Quality

global:  
  circuit\_breakers:  
    \# If system health degrades or budget risk increases, force COST\_LOW behavior.  
    auto\_fallback\_to\_cost\_low:  
      enabled: true  
      triggers:  
        \- type: "error\_rate"  
          window\_s: 60  
          threshold\_percent: 5  
        \- type: "webrtc\_fail\_rate"  
          window\_s: 120  
          threshold\_percent: 8  
        \- type: "estimated\_egress\_gb\_per\_hour\_total"  
          threshold\_gb: 500   \# should align with tier kill switches  
        \- type: "presence\_ws\_backlog"  
          threshold: 5000

  rate\_limits:  
    http\_requests:  
      per\_ip\_per\_minute: 600  
      per\_user\_per\_minute: 300  
    ws\_connections:  
      per\_ip: 20  
      per\_user: 6

  budgets:  
    \# Estimated egress monitoring (derived from active streams \* caps).  
    estimated\_egress\_gb\_per\_hour:  
      soft\_limit\_gb: 400  
      hard\_limit\_gb: 600  
      actions\_on\_soft:  
        \- "reduce\_presence\_tiles\_active"  
        \- "force\_thumb\_low\_profile"  
      actions\_on\_hard:  
        \- "force\_cost\_low\_everywhere"  
        \- "block\_new\_random\_queue\_joins"

presence:  
  ttl\_seconds: 25  
  heartbeat\_grace\_seconds: 10  
  max\_active\_tiles:  
    desktop: 12  
    mobile: 6  
  connect\_policy:  
    require\_viewport: true  
    allow\_audio: false  
  abuse\_protection:  
    max\_toggle\_cam\_per\_minute: 20  
    max\_presence\_mode\_changes\_per\_minute: 30

chat:  
  message\_limits:  
    per\_thread\_per\_user\_per\_minute: 20  
    per\_user\_global\_per\_minute: 60  
  spam\_controls:  
    enable\_link\_throttling: true  
    max\_links\_per\_minute: 2  
    max\_new\_threads\_per\_hour: 60  
  moderation:  
    report\_cooldown\_s: 20

sessions:  
  create\_limits:  
    per\_user\_per\_minute: 20  
    per\_ip\_per\_minute: 40  
  transition\_limits:  
    per\_user\_per\_minute: 30  
  consent:  
    required\_for\_private\_calls: true  
    invite\_timeout\_s: 20  
    max\_invites\_per\_hour: 120

random:  
  queue\_limits:  
    max\_joins\_per\_hour\_per\_user: 120  
    max\_concurrent\_searches\_per\_user: 1  
  skip\_controls:  
    min\_time\_before\_skip\_ms: 2000  
    max\_skips\_per\_minute: 12  
    cooldown\_schedule\_s: \[0, 0, 0, 3, 5, 8, 15, 30, 60\]  
  quarantine:  
    enabled: true  
    new\_account\_days: 3  
    \# Quarantine users get stricter caps and more cooldown.  
    quarantine\_overrides:  
      force\_media\_profile: "call\_random\_fast"  
      force\_upgrade\_wifi\_only: true  
      max\_matches\_per\_hour: 20  
      cooldown\_schedule\_s: \[0, 0, 3, 5, 8, 15, 30, 60, 120\]  
  enforcement\_actions:  
    on\_violation:  
      \- when: "skip\_spam"  
        actions: \["cooldown", "shadow\_quarantine"\]  
      \- when: "high\_report\_rate"  
        actions: \["cooldown", "force\_quarantine", "notify\_moderation\_queue"\]

webrtc:  
  reconnect\_budget\_hard\_max:  
    attempts\_per\_minute: 4  
  turn\_usage\_controls:  
    \# If TURN usage spikes (often indicates network issues), lower quality and reduce retries.  
    spike\_actions:  
      \- "force\_lower\_resolution"  
      \- "reduce\_reconnect\_attempts"  
  profiles\_hard\_caps:  
    \# Server-side caps; even if client requests more, enforce these.  
    thumb\_presence:  
      max\_width: 640  
      max\_height: 360  
      fps\_max: 15  
      bitrate\_kbps\_max: 500  
      audio\_enabled: false  
    call:  
      fps\_max: 30  
      bitrate\_kbps\_max: 3500

status:  
  health\_endpoints:  
    enable: true  
    routes:  
      \- "/status"  
  alerts:  
    enable: true  
    channels:  
      \- "dashboard"  
      \- "email\_optional"  
\`\`\`

\* \* \*

Se quiser, eu também te passo um \*\*exemplo de “deep-merge”\*\* (pseudo-código) e uma lista de \*\*validações automáticas\*\* pra rodar no CI (tipo “falha se alguém ligar áudio em thumbs” ou “tiles\\\_active \> hard max”).

\#\# Prompt:  
sim

\#\# Response:  
Perfeito — aqui vai: \*\*(1) exemplo de deep-merge (pseudo-código \+ TS)\*\* e \*\*(2) validações automáticas pra CI\*\* (com regras claras). Tudo feito pra “falhar cedo” e te proteger de custo/abuso. ✅

\* \* \*

1\) Deep-merge canônico  
\----------------------

\#\#\# 1.1 Regras (repetindo só o essencial)

\*   \*\*Objetos/dicts:\*\* merge recursivo por chave.  
\*   \*\*Listas:\*\* por padrão \*\*substitui a lista inteira\*\*.  
\*   \*\*Exceção:\*\* \`profiles.\*.webrtc.video.simulcast.layers\` pode ser mesclado por \`name\`.

\#\#\# 1.2 Pseudo-código (linguagem neutra)

\`\`\`  
function deepMerge(base, override, path):  
  if override is null: return base  
  if base is null: return override

  if base and override are dictionaries:  
    out \= copy(base)  
    for each key in override:  
      out\[key\] \= deepMerge(base\[key\], override\[key\], path \+ "." \+ key)  
    return out

  if base and override are lists:  
    if path endsWith ".simulcast.layers":  
      return mergeLayersByName(base, override)  
    else:  
      return override  \# replace whole list

  return override  
\`\`\`

\#\#\# 1.3 TypeScript (colável)

\`\`\`  
type AnyObj \= Record\<string, any\>;

function isObj(x: any): x is AnyObj {  
  return x \!== null && typeof x \=== "object" && \!Array.isArray(x);  
}

function mergeSimulcastLayersByName(base: any\[\], override: any\[\]) {  
  const map \= new Map\<string, AnyObj\>();  
  for (const layer of base || \[\]) {  
    if (layer?.name) map.set(layer.name, { ...layer });  
  }  
  for (const layer of override || \[\]) {  
    if (\!layer?.name) continue;  
    const prev \= map.get(layer.name) || {};  
    map.set(layer.name, deepMerge(prev, layer, ".layer")); // local merge  
  }  
  return Array.from(map.values());  
}

export function deepMerge(base: any, override: any, path \= ""): any {  
  if (override \=== undefined) return base;  
  if (base \=== undefined) return override;

  if (isObj(base) && isObj(override)) {  
    const out: AnyObj \= { ...base };  
    for (const \[k, v\] of Object.entries(override)) {  
      out\[k\] \= deepMerge(base\[k\], v, path ? \`${path}.${k}\` : k);  
    }  
    return out;  
  }

  if (Array.isArray(base) && Array.isArray(override)) {  
    if (path.endsWith("webrtc.video.simulcast.layers")) {  
      return mergeSimulcastLayersByName(base, override);  
    }  
    return override; // replace  
  }

  return override;  
}  
\`\`\`

\#\#\# 1.4 Pipeline de config (merge completo)

\`\`\`  
import { deepMerge } from "./deepMerge";

type ConfigBundle \= {  
  profiles: any;  
  modes: any;  
  routing: any;  
  shell: any;  
};

export function buildRuntimeConfig(  
  baseProfiles: any,  
  baseModes: any,  
  tierOverrides: any,  
  tierKillSwitches: any,  
  runtimeGuards: any  
) {  
  // 1\) base bundle  
  let cfg: any \= {  
    profiles: baseProfiles.profiles,  
    routing: baseModes.routing,  
    shell: baseModes.shell,  
    modes: baseModes.modes  
  };

  // 2\) apply tier overrides  
  cfg \= deepMerge(cfg, tierOverrides, "");

  // 3\) validate invariants (throw if invalid)  
  validateConfig(cfg);

  // 4\) apply enforcement layers (do not increase limits)  
  cfg \= applyKillSwitches(cfg, tierKillSwitches);  
  cfg \= applyRuntimeGuards(cfg, runtimeGuards);

  // 5\) validate again after enforcement  
  validateConfig(cfg);

  return cfg as ConfigBundle;  
}  
\`\`\`

\> \`applyKillSwitches/applyRuntimeGuards\` são “clamps”: eles \*\*reduzem\*\* limites, nunca aumentam.

\* \* \*

2\) Validações automáticas (CI) — lista de checks  
\------------------------------------------------

Abaixo estão os checks que eu recomendo como \*\*mínimo obrigatório\*\*. O CI falha se qualquer um quebrar.

\#\#\# 2.1 Presença (Party/Sniffers) — custo e privacidade

\*   \*\*Audio em thumbs proibido\*\*  
    \*   \`profiles.thumb\_\* webrtc.audio.enabled \=== false\`  
\*   \*\*Limite de tiles ativos\*\*  
    \*   \`modes.party.ui\_limits.tiles\_active.desktop \<= 20\`  
    \*   \`modes.party.ui\_limits.tiles\_active.mobile \<= 10\`  
    \*   \`modes.sniffers.ui\_limits.tiles\_active.desktop \<= 20\`  
    \*   \`modes.sniffers.ui\_limits.tiles\_active.mobile \<= 10\`  
\*   \*\*Gating obrigatório\*\*  
    \*   \`connect\_strategy in \["viewport\_only", "on\_demand"\]\`

\#\#\# 2.2 Random — anti abuso \+ anti custo

\*   \`min\_time\_before\_skip\_ms \>= 1000\`  
\*   \`max\_skips\_per\_minute\` (do runtime-guards) \<= 20  
\*   \`reconnect\_budget.attempts\_per\_minute \<= 4\`  
\*   \`cooldown\_schedule\_s\`:  
    \*   tem pelo menos 5 itens  
    \*   após índice 3: é não-decrescente  
\*   \`quarantine.enabled \=== true\` (no MVP)

\#\#\# 2.3 Separação de ecossistemas

\*   \`modes.live\_public.policies.separate\_ecosystem\_from\_party \=== true\`  
\*   \`modes.party.rail.component \=== presence\_rail\`  
\*   \`modes.live\_public.rail.component \!== presence\_rail\`

\#\#\# 2.4 WebRTC hard caps (segurança)

\*   thumbs:  
    \*   \`fps\_max \<= 15\`  
    \*   \`bitrate\_kbps\_max \<= 500\`  
    \*   \`max\_height \<= 360\` (ou o teto que você definir)  
\*   calls:  
    \*   \`fps\_max \<= 30\`  
    \*   \`bitrate\_kbps\_max \<= 3500\` (hard cap)  
\*   meeting:  
    \*   \`caps\_by\_participants\` deve existir e cobrir “1-4”, “5-8”, “9-99”

\#\#\# 2.5 Broadcast ladder (MVP)

\*   \`profiles.broadcast\_hls\_lab.hls.ladder.length \<= 4\`  
\*   \`startup\_buffer\_s\` entre 2 e 6

\#\#\# 2.6 Integridade de referências

\*   Todo \`media\_profile\` usado em \`mode-manifests.yaml\` existe em \`media-profiles.yaml\`  
\*   Toda action que referencia \`call\_media\_profile\` existe  
\*   Rotas não duplicadas

\* \* \*

3\) Exemplo de validador (TypeScript, colável)  
\---------------------------------------------

\`\`\`  
function assert(cond: any, msg: string) {  
  if (\!cond) throw new Error(\`CONFIG\_INVALID: ${msg}\`);  
}

function isNonDecreasing(arr: number\[\], startIdx: number) {  
  for (let i \= Math.max(1, startIdx); i \< arr.length; i++) {  
    if (arr\[i\] \< arr\[i \- 1\]) return false;  
  }  
  return true;  
}

export function validateConfig(cfg: any) {  
  // Reference integrity  
  const profiles \= cfg.profiles || {};  
  const modes \= cfg.modes || {};

  const mustHaveProfile \= (name: string, where: string) \=\>  
    assert(\!\!profiles\[name\], \`${where} references missing media profile "${name}"\`);

  // \--- Presence profiles \---  
  if (profiles.thumb\_low?.webrtc) {  
    assert(profiles.thumb\_low.webrtc.audio?.enabled \=== false, "thumb\_low must have audio disabled");  
  }

  // \--- Party limits \---  
  const party \= modes.party;  
  assert(\!\!party, "party mode missing");  
  assert(\["viewport\_only", "on\_demand"\].includes(party.ui\_limits?.connect\_strategy), "party connect\_strategy must gate streams");  
  assert(party.ui\_limits?.tiles\_active?.desktop \<= 20, "party tiles\_active.desktop too high");  
  assert(party.ui\_limits?.tiles\_active?.mobile \<= 10, "party tiles\_active.mobile too high");  
  mustHaveProfile(party.rail?.tile\_media\_profile, "party.rail.tile\_media\_profile");  
  mustHaveProfile(party.stage?.media\_profile, "party.stage.media\_profile");

  // \--- Sniffers limits \---  
  const sniffers \= modes.sniffers;  
  assert(\!\!sniffers, "sniffers mode missing");  
  assert(\["viewport\_only", "on\_demand"\].includes(sniffers.ui\_limits?.connect\_strategy), "sniffers connect\_strategy must gate streams");  
  assert(sniffers.ui\_limits?.tiles\_active?.desktop \<= 20, "sniffers tiles\_active.desktop too high");  
  assert(sniffers.ui\_limits?.tiles\_active?.mobile \<= 10, "sniffers tiles\_active.mobile too high");  
  mustHaveProfile(sniffers.stage?.tile\_media\_profile, "sniffers.stage.tile\_media\_profile");

  // \--- Random constraints \---  
  const random \= modes.random;  
  assert(\!\!random, "random mode missing");  
  mustHaveProfile(random.stage?.call\_media\_profile, "random.stage.call\_media\_profile");  
  assert(random.matching?.min\_time\_before\_skip\_ms \>= 1000, "random min\_time\_before\_skip\_ms too low");  
  assert(random.matching?.reconnect\_budget?.attempts\_per\_minute \<= 4, "random reconnect\_budget too high");  
  const cds \= random.matching?.cooldown\_schedule\_s || \[\];  
  assert(Array.isArray(cds) && cds.length \>= 5, "random cooldown\_schedule\_s too short");  
  assert(isNonDecreasing(cds, 3), "random cooldown\_schedule\_s must be non-decreasing after index 3");  
  assert(random.matching?.quarantine?.enabled \=== true, "random quarantine must be enabled");

  // \--- Separation \---  
  const live \= modes.live\_public;  
  if (live) {  
    assert(live.policies?.separate\_ecosystem\_from\_party \=== true, "live\_public must be separate from party");  
    assert(live.rail?.component \!== "presence\_rail", "live\_public must not use presence\_rail");  
  }

  // \--- Broadcast ladder \---  
  const b \= profiles.broadcast\_hls\_lab?.hls;  
  if (b) {  
    assert((b.ladder?.length || 0\) \<= 4, "broadcast ladder too large for MVP");  
    const sb \= b.player?.startup\_buffer\_s;  
    if (sb \!== undefined) assert(sb \>= 2 && sb \<= 6, "broadcast startup\_buffer\_s out of range");  
  }

  return true;  
}  
\`\`\`

\* \* \*

4\) “Clamps” (enforcement) — conceito rápido  
\-------------------------------------------

A regra é: \*\*tier \+ guards só podem reduzir limites\*\*, nunca aumentar.    
Exemplo: se UI pedir 16 tiles ativos mas guard diz 12, fica 12\.

Se quiser, eu te mando também um \`applyRuntimeGuards()\` pr  
