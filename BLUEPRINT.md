voulezvous.tv

Como funciona, do macro ao micro (versão detalhada)

1) O que é a VVTV

Uma TV 24/7 de filmes adultos que se abastece sozinha. Ela:
	•	procura vídeos e músicas na internet;
	•	avalia se combinam com o estilo do canal;
	•	planeja a grade com antecedência (sem baixar tudo de cara);
	•	baixa apenas o que realmente vai ao ar;
	•	prepara e checa a qualidade;
	•	transmite ininterruptamente;
	•	aprende um pouco por dia para ficar mais afinada com o gosto do canal.

Tudo guiado por um “Cartão do Dono” — um documento de políticas editoriais e operacionais que define como a TV deve se comportar.

⸻

2) O papel do “Cartão do Dono”

Pense nele como um conjunto de regras que a máquina segue fielmente. Exemplos:
	•	Estilo e ritmo: duração média dos vídeos, equilíbrio entre temas, variedade de estética.
	•	Busca: que palavras priorizar, que fontes evitar, quando insistir e quando desistir.
	•	Agendamento: com quantas horas de antecedência baixar, qual a “folga” mínima de conteúdo pronto.
	•	Qualidade: resolução mínima, volume padrão de áudio, critérios de rejeição.
	•	Música: porcentagem de blocos com trilhas específicas, climas prioritários.
	•	Liberdade de curadoria: o quanto a TV pode “reordenar” a fila por conta própria.

Você muda o Cartão quando quiser; a TV aplica imediatamente. Ajustes finos do dia a dia podem ser feitos automaticamente pelo sistema (com registro e limites).

⸻

3) O ciclo diário (linha do tempo)

A) Descobrir (o robô “navega como gente”)
	•	Entra em sites de vídeo.
	•	Dá play de verdade para confirmar a versão em HD (muitos sites só liberam HD após o play).
	•	Captura link, título, duração, pistas visuais/temáticas e “sinaizinhos” de qualidade.
	•	Se protege de anúncios agressivos, pop-ups, links falsos e rastreadores.

Saída desta etapa: uma lista de PLANOS (intenções) com todos os detalhes, sem baixar os arquivos ainda.

B) Planejar (sem ocupar disco à toa)
	•	Com base no Cartão do Dono, a TV monta a biblioteca de planos do dia: quais vídeos podem entrar, quais são reservas, quais combinam com certos horários.
	•	Garante variedade (não repete cor/tema/posição de câmera por longos blocos).
	•	Reserva espaço para música e vinhetas quando for o caso.

Vantagem: armazenamento é caro; planos são leves. Decidir cedo, baixar tarde.

C) Confirmar e baixar só o necessário (T-4h)
	•	Faltando poucas horas para ir ao ar, a TV revisa a biblioteca de planos e decide o que realmente baixar.
	•	Se algum link tiver mudado, tenta caminhos alternativos (ou aciona reservas).
	•	Baixa o arquivo correto (o mesmo que estava tocando no site, em HD).

D) Preparar e checar qualidade
	•	Ajusta o contêiner (sem mudar o conteúdo).
	•	Normaliza o volume do áudio para um padrão confortável.
	•	Confere resolução, nitidez e integridade.
	•	Arquivos que não passam nos critérios são trocados por reservas.

E) Fila e transmissão (com folga)
	•	Monta a fila do dia e mantém sempre ~1 hora de folga já pronta.
	•	Se o buffer cair (falta de conteúdo), ativa o modo emergência com vídeos de reserva para não derrubar a transmissão.
	•	Publica o streaming (HLS), que qualquer pessoa pode assistir de casa no link do canal.

F) Curadoria “com bom senso”
	•	Um “curador” inteligente observa tudo: se ficou repetitivo, se os blocos estão longos, se a paleta de cores saturou.
	•	Ele sugere trocas simples (ex.: “trocar vídeo #4 por #7 para quebrar repetição”).
	•	Quando a confiança é alta e as políticas permitem, ele aplica a troca; caso contrário, só aconselha.
	•	Todas as sugestões e ações ficam registradas, para você auditar depois.

G) Aprender com o dia (de madrugada)
	•	A TV analisa o que foi ao ar, como ficou a qualidade e a diversidade, e faz ajustes finos automáticos dentro de limites (por exemplo, aumentar um pouquinho a variedade no dia seguinte).
	•	As mudanças grandes você faz quando quiser — tipicamente 1 vez por mês.

⸻

4) O que roda “em casa” vs “na nuvem”

No seu LAB (Mac mini)
	•	Navegação real nos sites (para achar a versão HD).
	•	Geração e gestão dos PLANOS.
	•	Downloads, preparação e checagem.
	•	Fila e transmissão 24/7.
	•	“Curador” de bom senso (IA local, sem bloqueios/censura).

Na nuvem (Cloudflare)
	•	API leve de controle (painel, comandos simples).
	•	Relatórios e histórico resumido (sem dados sensíveis).
	•	Página de status/monitoramento.

Esse desenho garante privacidade, baixa latência nas partes críticas e simplicidade nas interfaces externas.

⸻

5) Segurança e “armadura” contra a web adulta
	•	Navegação protegida: bloqueio de rastreadores e domínios suspeitos, isca anti-clickbait, tentativa de isolar anúncios.
	•	Confirmação de vídeo real: a TV só considera “válida” a fonte que tocou em HD.
	•	Listas do que evitar: domínios barrados, palavras proibidas, marcas d’água suspeitas.
	•	Registro de evidências: o que foi visto, de onde veio, por que foi escolhido — tudo com horário, para auditoria.

⸻

6) Como você controla (sem virar escravo do sistema)
	•	Você define as políticas no Cartão do Dono: estilo, ritmo, variedade, horários, música, liberdade do curador etc.
	•	Você acompanha relatórios: a TV gera resumos diários/semanais que mostram o que entrou, por que entrou e o que poderia ter sido melhor.
	•	Você muda quando quiser, mas não precisa mexer todo dia. O sistema se ajusta sozinho no detalhe; você dá o rumo.

⸻

7) O que a TV registra (e por quê)
	•	Planos feitos (sem baixar): o que estava previsto.
	•	Decisões de baixar (T-4h): o que virou real.
	•	Checagens de qualidade: o que passou/reprovou e por quê.
	•	Fila/Transmissão: o que foi ao ar e quando.
	•	Sugestões do curador: o que ele viu, o que sugeriu, o que foi aplicado.
	•	Ajustes automáticos: mudanças finas para o dia seguinte.

Isso garante rastreamento completo sem burocracia.

⸻

8) Exemplo de dia (resumo prático)
	•	10:00 • Robô encontra 60 vídeos, valida o HD ao dar play, gera PLANOS.
	•	12:00 • Planejamento monta a biblioteca do dia, já com reservas.
	•	17:30 (T-4h) • Decide 18 itens para o bloco noturno; baixa só esses.
	•	18:30 • Prepara/checa, monta a fila com 1h de folga.
	•	19:00—23:00 • Transmissão; curador sugere troca pontual para quebrar repetição.
	•	03:00 • Ajustes finos automáticos com base no dia (sem mexer no seu estilo).

⸻

9) O que é “bom o suficiente” para ligar agora
	•	Robô de busca com confirmação de HD e proteções de navegação.
	•	Planejador que gera PLANOS sem baixar.
	•	Esteira de baixar T-4h → preparar → checar → enfileirar.
	•	Transmissão com folga e modo emergência.
	•	Curador local com sugestões e aplicação limitada.
	•	Rotina da madrugada com pequenos ajustes automáticos e relatório.

Com isso, o canal já fica estável, melhora um pouco por dia e você mantém o controle de rumo.

⸻

10) Em uma frase

A VVTV é uma TV que trabalha sozinha, economiza onde importa, garante qualidade, mantém a programação viva e aprende com a prática — tudo seguindo as suas regras.

# Blueprint Técnico VVTV (Rust-first, MVP executável + base de evolução)

## Resumo
Implementar a VVTV como um sistema híbrido `LAB + Cloudflare` com operação 24/7, descoberta externa controlada por `allowlist`, distribuição `HLS`, curadoria `automática total` dentro de limites do Cartão do Dono, SLO inicial de `99.5%`, retenção de auditoria de `90 dias` e custo de nuvem `baixo`.

## Escopo fechado (in/out)
1. Incluir no MVP:
- Descoberta web com validação de playback real em HD.
- Planejamento por PLANOS sem download antecipado.
- Janela T-4h para decisão final e download seletivo.
- Esteira preparar/normalizar/QA/enfileirar.
- Transmissão HLS contínua com buffer-alvo de 1h e modo emergência.
- Curador com aplicação automática auditável.
- Ajuste fino diário automático com guardrails.
- API leve Cloudflare para controle, status e relatórios resumidos.

2. Fora do MVP:
- Multi-protocolo (RTMP/SRT etc.).
- App de edição avançada de programação.
- Recomendação por feedback de audiência em tempo real.
- Multi-node ativo-ativo no LAB.

## Arquitetura alvo (componentes)
1. LAB (Mac mini, Rust):
- `vvtv-discovery`: browser automation hardened para coleta e validação HD.
- `vvtv-planner`: geração de PLANOS, reservas e grade preliminar.
- `vvtv-fetcher`: resolução final T-4h, fallback de links, download.
- `vvtv-prep`: remux + normalização de áudio + checks técnicos.
- `vvtv-queue`: montagem de playlist de execução e controle de buffer.
- `vvtv-stream`: empacotamento HLS e publicação local.
- `vvtv-curator`: reordenação/substituição automática conforme política.
- `vvtv-nightly`: ajuste fino de pesos/regras dentro de limites.
- `vvtv-control-agent`: cliente da API Cloudflare + sync de estado.
- `vvtv-audit`: trilha imutável de eventos e decisões.

2. Nuvem (Cloudflare):
- Worker API de controle.
- D1 para metadados resumidos de operação.
- R2 opcional para snapshots de relatório.
- Dashboard/status page simples (read-only + comandos mínimos).

3. Fluxo principal:
- Discover -> Plan -> Commit(T-4h) -> Fetch -> Prep/QA -> Queue -> Stream -> Curate -> Nightly Learn.

## Contratos públicos e tipos (decisão completa)
1. `OwnerCard` (fonte única de política, versionada):
- Campos obrigatórios: `editorial_profile`, `search_policy`, `schedule_policy`, `quality_policy`, `music_policy`, `curator_policy`, `safety_policy`, `autotune_policy`.
- Formato: `YAML` versionado com `schema_version`.
- Aplicação: hot-reload com validação; falha mantém versão anterior.

2. `PlanItem` (intenção sem arquivo):
- Identidade: `plan_id`, `source_url`, `source_domain`, `discovered_at`.
- Conteúdo: `title`, `duration_sec`, `theme_tags`, `visual_features`, `quality_signals`.
- Estado: `CANDIDATE | RESERVED | SCHEDULED | COMMITTED | DROPPED`.
- Justificativa: `selection_reason`, `policy_match_score`.

3. `AssetItem` (arquivo confirmado):
- Campos: `asset_id`, `plan_id`, `local_path`, `checksum`, `resolution`, `audio_lufs`, `qa_status`.

4. `QueueEntry`:
- Campos: `entry_id`, `asset_id`, `start_at`, `slot_type`, `fallback_level`, `curation_trace_id`.

5. `AuditEvent`:
- Campos: `event_id`, `ts`, `actor`, `module`, `action`, `before`, `after`, `decision_score`, `reason_code`.

6. API Cloudflare (`/v1`):
- `GET /status` estado operacional e buffer atual.
- `GET /reports/daily?date=YYYY-MM-DD` resumo diário.
- `GET /reports/weekly?week=YYYY-Www` resumo semanal.
- `POST /control/reload-owner-card` recarregar políticas.
- `POST /control/emergency-mode` ligar/desligar modo emergência.
- `POST /control/curator-mode` setar agressividade dentro dos limites permitidos.

## Regras operacionais fechadas
1. Descoberta:
- Apenas domínios em allowlist.
- Requer evento de playback real para validar fonte.
- Rejeição imediata por blacklist/sinais suspeitos configurados.

2. Planejamento:
- Horizonte de planejamento de 24h.
- Diversidade mínima por blocos conforme thresholds do OwnerCard.
- Reserva técnica mínima de candidatos por janela horária.

3. Commit T-4h:
- Job a cada 30 min selecionando itens para janelas que começam em até 4h.
- Link inválido aciona retries e fallback para reserva.

4. Qualidade:
- Gate técnico obrigatório antes de ir para fila.
- Qualquer reprovação substitui item por reserva sem interromper pipeline.

5. Buffer/continuidade:
- Alvo de buffer pronto: 60 min.
- Threshold crítico: <20 min ativa emergência.
- Emergência usa pool pré-validado de segurança.

6. Curadoria automática total:
- Pode aplicar trocas/reordenação sem aprovação humana.
- Guardrails: não violar diversidade mínima, limites de repetição e categorias proibidas.
- Todas as mudanças geram `AuditEvent` com motivo e score.

7. Nightly learning:
- Execução 03:00 local.
- Ajustes apenas em pesos finos com limite percentual diário.
- Mudanças estruturais nunca automáticas.

## Plano de implementação (ordem obrigatória)
1. Fundação:
- Criar monorepo Rust workspace com crates por módulo.
- Definir schemas `OwnerCard`, `PlanItem`, `AssetItem`, `QueueEntry`, `AuditEvent`.
- Implementar camada de configuração + validação + hot-reload.

2. Pipeline base:
- Implementar `discovery -> planner -> fetcher -> prep -> queue -> stream` ponta a ponta com mocks.
- Substituir mocks por integrações reais incrementalmente.

3. Segurança e robustez:
- Harden de navegação, allowlist/blacklist, retries/backoff, fallback chain.
- Idempotência por `plan_id` e deduplicação por checksum/source fingerprint.

4. Curadoria e aprendizado:
- Implementar motor de score de repetição/diversidade.
- Habilitar aplicação automática com trilha completa.
- Implementar rotina nightly com limites de ajuste.

5. Controle e observabilidade:
- Subir API Cloudflare mínima e sincronização de estado do LAB.
- Expor métricas, health endpoints e relatórios diário/semanal.

6. Operação:
- Criar runbooks de incidentes.
- Definir backup/restore de metadados.
- Entrar em canary interno e depois operação contínua.

## Testes e critérios de aceite
1. Testes de contrato:
- Validação de schema `OwnerCard` com versões compatíveis.
- Serialização/desserialização estável dos tipos centrais.

2. Testes de fluxo:
- Cenário feliz completo de 24h simulado.
- Link quebrado no T-4h com fallback automático.
- Reprovação de QA com substituição por reserva.
- Queda de buffer com entrada/saída de emergência.

3. Testes de curadoria:
- Detectar repetição e aplicar troca sem violar políticas.
- Garantir log de decisão completo em 100% das ações automáticas.

4. Testes de resiliência:
- Reinício de processo com recuperação de estado.
- Falha de rede temporária sem parada de transmissão.
- Falha de API Cloudflare sem impacto no core local.

5. Critérios de aceite MVP:
- Uptime mensal >= 99.5%.
- Zero interrupção por falta de conteúdo em cenário nominal.
- 100% dos itens no ar com registro de origem/decisão/QA.
- Relatório diário disponível até 06:00 local.

## Observabilidade e operação
1. Métricas mínimas:
- `buffer_minutes`, `plans_created`, `plans_committed`, `qa_pass_rate`, `fallback_rate`, `curator_actions`, `stream_disruptions`.

2. Logs estruturados:
- JSON com `trace_id` por item do pipeline.
- Retenção local + resumo em nuvem por 90 dias.

3. Alertas:
- Buffer crítico.
- Queda de taxa de QA.
- Crescimento anormal de fallback.
- Falha contínua em descoberta por domínio.

## Assumptions e defaults adotados
1. Workspace atual está vazio; o plano assume implementação greenfield.
2. Distribuição inicial será somente HLS.
3. Descoberta externa seguirá allowlist administrada por você.
4. Curadoria automática total permanece limitada por guardrails do OwnerCard.
5. Nuvem terá papel leve (controle/status/relatórios), sem mover o core do pipeline para fora do LAB.
6. Sem decisão explícita de DRM, geoblock ou monetização no MVP.
