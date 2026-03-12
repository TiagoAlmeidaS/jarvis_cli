# Issues sugeridas: Fechar o loop autônomo (Gaps G1–G6)

Objetivo: validar e completar as entregas dos gaps G1–G6 do [autonomy-roadmap](../architecture/autonomy-roadmap.md), para que o daemon observe → analise → decida → **atue** e o TUI seja tão capaz quanto Claude Code / Cursor.

Use este doc para criar as issues no seu repositório (GitHub/GitLab). Cada bloco abaixo é uma issue pronta para copiar. Componentes existentes: daemon (ProposalExecutor, Goals, scheduler), core (issue resolver), [docs/features](../features/) (proposal-executor, goal-system, real-data-integration, tool-calling-native, agentic-loop, sandbox-execution).

---

## Issue G1 — Validar Proposal Executor em ambiente real

**Título:** `test(daemon): validar que propostas aprovadas são executadas em ambiente real`

**Descrição:**

O [Proposal Executor](../features/proposal-executor.md) está implementado no código (`daemon/src/executor.rs`). Garantir que, em ambiente real (ou E2E), propostas aprovadas são de fato executadas e o loop "Strategy Analyzer → Proposals → Executor → Ação" fecha.

- Validar fluxo: proposta aprovada (via CLI ou Yolo) → executor executa a action type correspondente → estado do sistema alterado conforme esperado.
- Documentar passos de teste manual ou adicionar teste automatizado que cubra pelo menos um action type.
- Não duplicar lógica: o executor já existe; foco em validação e documentação.

**Critério de aceite:**

- Existe teste ou runbook que demonstra proposta aprovada sendo executada com sucesso.
- Nenhum regresso em cenários com OpenRouter/Google.

**Labels sugeridas:** `testing`, `daemon`, `autonomy`

---

## Issue G2 — Validar Goal System e integração com strategy_analyzer

**Título:** `test(daemon): validar goals configurados e uso pelo strategy_analyzer`

**Descrição:**

O [Goal System](../features/goal-system.md) está implementado (tabela `daemon_goals`, bootstrap). Garantir que goals estão configurados e que o `strategy_analyzer` os usa para priorizar propostas.

- Validar que goals podem ser criados/listados via CLI ou DB.
- Validar que o strategy_analyzer recebe ou consulta goals ao gerar propostas (priorização ou filtro).
- Documentar exemplo de configuração de goals (ex.: revenue >= X, content_count >= Y) e resultado esperado no analyzer.

**Critério de aceite:**

- Goals configuráveis e refletidos no comportamento do strategy_analyzer (documentado ou coberto por teste).
- Referência ao [goal-system.md](../features/goal-system.md) na doc.

**Labels sugeridas:** `testing`, `daemon`, `autonomy`

---

## Issue G3a — Real Data: WordPress Stats API

**Título:** `feat(daemon): integrar WordPress Stats API para métricas reais (G3)`

**Descrição:**

Implementar ou completar a integração com WordPress REST API para pageviews/dados reais, conforme [real-data-integration.md](../features/real-data-integration.md) (Fonte 2). O daemon hoje estima métricas; com dados reais o strategy_analyzer toma decisões úteis.

- Usar WordPress REST API (ou plugin de stats) para obter pageviews por post.
- Alimentar metrics collector ou goals com dados reais.
- Documentar config (URL, credenciais) e cenário de teste.

**Critério de aceite:**

- Métricas de WordPress (ex.: pageviews) disponíveis para o daemon; strategy_analyzer ou goals podem usar esses dados.
- Doc atualizada em real-data-integration ou DAEMON_QUICK_START.

**Labels sugeridas:** `feature`, `daemon`, `autonomy`

---

## Issue G3b — Real Data: Revenue manual input CLI

**Título:** `feat(daemon): comando CLI para input manual de revenue (G3)`

**Descrição:**

Implementar comandos de input manual de revenue e métricas conforme [real-data-integration.md](../features/real-data-integration.md) (Fonte 1, Step 1.4). Exemplo: `jarvis daemon revenue add 15.50 --source adsense --period 2026-02`.

- Comandos: `revenue add`, opcionalmente `metrics add` para pageviews manuais.
- Persistir em SQLite (daemon-common); goals e strategy_analyzer passam a usar dados reais quando disponíveis.
- Documentar em DAEMON_QUICK_START ou real-data-integration.

**Critério de aceite:**

- Usuário pode registrar revenue (e opcionalmente métricas) via CLI; dados persistem e são usados pelo daemon.
- Doc atualizada.

**Labels sugeridas:** `feature`, `daemon`, `autonomy`

---

## Issue G3c — Real Data: Google Search Console e AdSense API

**Título:** `feat(daemon): integrar GSC e AdSense API para dados reais (G3)`

**Descrição:**

Integrar Google Search Console e Google AdSense APIs conforme [real-data-integration.md](../features/real-data-integration.md) (Fontes 3 e 4). O daemon já tem referência a data sources (GSC, AdSense); completar integração e expor dados ao metrics collector / goals.

- GSC: clicks, impressões, CTR, posição por página/query.
- AdSense: earnings por página, RPM.
- OAuth2 e config documentados; dados persistidos ou disponíveis para strategy_analyzer.

**Critério de aceite:**

- Dados reais de GSC e AdSense disponíveis para o daemon (metrics collector ou goals).
- Doc e config documentados.

**Labels sugeridas:** `feature`, `daemon`, `autonomy`

---

## Issue G4 — Tool Calling Nativo (client-side)

**Título:** `feat(tui): tool calling nativo client-side (G4)`

**Descrição:**

Implementar ou completar o [Tool Calling Nativo](../features/tool-calling-native.md): o client (TUI/CLI) gerencia o registro de tools e o dispatch, e o modelo retorna tool_use; o client executa e devolve o resultado. Assim o TUI funciona com modelos baratos que não suportam function calling de forma robusta.

- Tool registry + client-side dispatcher conforme a doc.
- Parse de resposta do modelo para tool_use; execução da tool; injeção do resultado no contexto.
- Referência ao [tool-calling-native.md](../features/tool-calling-native.md) e arquivos em `jarvis-rs/core` (tools, TUI).

**Critério de aceite:**

- TUI pode executar tools (ex.: shell, file) com modelo que retorna tool_use, sem depender de function calling nativo do modelo.
- Doc atualizada.

**Labels sugeridas:** `feature`, `tui`, `autonomy`

---

## Issue G5 — Agentic Loop completo (think → execute → observe)

**Título:** `feat(tui): agentic loop completo no client (G5)`

**Descrição:**

Implementar o [Agentic Loop](../features/agentic-loop.md) no client: ciclo Think → Execute → Observe → Repeat gerenciado pelo TUI/CLI, não delegado totalmente ao modelo. Inclui observação de resultado e re-planejamento.

- Loop explícito: planejar → executar tool → observar saída → decidir próximo passo (ou concluir).
- Pode depender de G4 (tool calling nativo) para execução client-side.
- Referência a [agentic-loop.md](../features/agentic-loop.md).

**Critério de aceite:**

- Ciclo observe → decide → act visível no TUI; re-planejamento após execução de tool quando aplicável.
- Doc atualizada.

**Labels sugeridas:** `feature`, `tui`, `autonomy`

---

## Issue G6 — Sandbox execution robusto (rollback, limites)

**Título:** `feat(core): sandbox execution robusto com rollback e limites (G6)`

**Descrição:**

Reforçar o [Sandbox Execution](../features/sandbox-execution.md): isolamento seguro, rollback em falha, limites de recursos e trilha de auditoria para ações autônomas. Reduz risco de ações destrutivas.

- Rollback: reverter alterações de arquivo ou estado quando uma ação falha.
- Limites: tempo, memória, rede (conforme doc).
- Audit trail: log de ações executadas no sandbox.
- Referência a [sandbox-execution.md](../features/sandbox-execution.md) e código em `jarvis-rs/core` (sandboxing).

**Critério de aceite:**

- Sandbox com rollback e limites documentados e implementados; audit trail disponível.
- Nenhum regresso em execução de comandos atuais.

**Labels sugeridas:** `feature`, `core`, `autonomy`

---

## Ordem sugerida

| Ordem | Issue | Motivo |
|-------|--------|--------|
| 1 | G1 (Proposal Executor) | Fechar loop proposta → execução. |
| 2 | G2 (Goal System) | Direção para o analyzer. |
| 3 | G3a, G3b (WordPress + revenue manual) | Dados reais mínimos. |
| 4 | G3c (GSC, AdSense) | Dados reais completos. |
| 5 | G4, G5 (Tool calling, Agentic loop) | TUI independente do modelo. |
| 6 | G6 (Sandbox) | Segurança em autonomia. |

---

**Última atualização:** 2026-03-11
