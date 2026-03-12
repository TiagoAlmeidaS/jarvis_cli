# Issues sugeridas: Crescimento e auto-aprendizado

Objetivo: fechar o ciclo de **aprendizado** do Jarvis: dados reais alimentando métricas e goals, A/B e otimização de prompt com dados reais, integração do módulo autonomous do core ao daemon, e documentação do feedback loop (métricas → analyzer → propostas → executor → métricas). Fontes: [autonomy-roadmap](../architecture/autonomy-roadmap.md) (Fase 3: Inteligência avançada), pipelines existentes (strategy_analyzer, prompt_optimizer, ab_tester), [autonomous-architecture-phase3](../features/autonomous-architecture-phase3.md) (RAG/knowledge).

Use este doc para criar as issues no seu repositório (GitHub/GitLab). Cada bloco abaixo é uma issue pronta para copiar. **Não duplicar decisão no daemon**; o core/autonomous pode ser invocado pelo daemon.

---

## Issue A1 — Dados reais (GSC, AdSense) no metrics collector e goals

**Título:** `feat(daemon): integrar GSC e AdSense ao metrics collector e goals para dados reais`

**Descrição:**

Conectar as APIs reais de Google Search Console e AdSense (já referenciadas em [real-data-integration](../features/real-data-integration.md) e data sources do daemon) ao metrics collector e ao Goal System. Assim o strategy_analyzer e os goals passam a usar dados reais em vez de estimativas.

- Metrics collector: consumir dados de GSC (clicks, impressões, CTR) e AdSense (revenue, RPM) e persistir ou expor para pipelines.
- Goals: permitir metas baseadas em métricas reais (ex.: revenue >= X, pageviews >= Y) usando esses dados.
- Documentar configuração (OAuth, fontes) e fluxo de atualização.

**Critério de aceite:**

- Dados reais de GSC e AdSense alimentam metrics collector e goals; strategy_analyzer pode usar esses dados nas propostas.
- Doc atualizada (real-data-integration ou daemon).

**Labels sugeridas:** `feature`, `daemon`, `autonomy`

---

## Issue A2 — A/B testing e prompt_optimizer com dados reais

**Título:** `feat(daemon): A/B testing de títulos SEO e prompt_optimizer com dados reais`

**Descrição:**

Usar os pipelines existentes **ab_tester** e **prompt_optimizer** com dados reais (métricas do metrics collector / GSC/AdSense). Permitir A/B de títulos ou conteúdo SEO e otimização de prompts com base em resultados medidos.

- A/B testing: pipeline ab_tester usando métricas reais (CTR, pageviews, revenue) para comparar variantes.
- prompt_optimizer: alimentar com resultados reais (conversão, engagement) para ajustar prompts de geração.
- Documentar como configurar experimentos e como os dados reais entram no ciclo.

**Critério de aceite:**

- A/B de títulos (ou equivalente) executável com métricas reais; prompt_optimizer pode usar dados reais quando disponíveis.
- Doc descreve fluxo A/B e otimização.

**Labels sugeridas:** `feature`, `daemon`, `autonomy`

---

## Issue A3 — Integrar core/autonomous ao fluxo do daemon

**Título:** `feat(daemon,core): integrar módulo autonomous do core ao fluxo do daemon (scheduler/strategy_analyzer)`

**Descrição:**

O core possui módulo de arquitetura autônoma (ex.: `core/src/autonomous/` ou equivalente, RAG/knowledge em [autonomous-architecture-phase3](../features/autonomous-architecture-phase3.md)). Integrar esse módulo ao fluxo do daemon para que o strategy_analyzer ou o scheduler possam usar contexto RAG e conhecimento sem duplicar lógica de decisão no daemon.

- Definir ponto de integração: daemon invoca core (RAG retriever, knowledge base) para enriquecer contexto antes de strategy_analyzer ou em pipelines específicos.
- Não duplicar: decisão continua no core ou no analyzer que já usa LLM; daemon apenas orquestra e passa contexto.
- Documentar onde RAG/knowledge entra no pipeline (ex.: análise de estratégia, resolução de issues).

**Critério de aceite:**

- Daemon pode usar RAG/knowledge do core em pelo menos um pipeline (ex.: strategy_analyzer); doc descreve a integração.
- Nenhuma duplicação de lógica de decisão no daemon.

**Labels sugeridas:** `feature`, `daemon`, `core`, `autonomy`

---

## Issue A4 — Documentar feedback loop e onde entra aprendizado

**Título:** `docs: documentar ciclo métricas → analyzer → propostas → executor e onde entra aprendizado`

**Descrição:**

Documentar o ciclo completo de feedback e onde "aprendizado" entra: métricas coletadas → strategy_analyzer → propostas → executor → novas métricas. Incluir goals, experiments (A/B) e prompt_optimizer como mecanismos de aprendizado.

- Diagrama ou doc: fluxo "métricas → analyzer → propostas → executor → métricas" e pontos em que goals, experiments e prompt_optimizer influenciam decisões.
- Onde o aprendizado é persistido (goals atualizados, experiments, cache de prompts) e como isso melhora ciclos futuros.
- Referência a [autonomy-roadmap](../architecture/autonomy-roadmap.md) e [autonomous-architecture-phase3](../features/autonomous-architecture-phase3.md).

**Critério de aceite:**

- Doc publicada (ex.: em docs/architecture ou docs/features) descrevendo o ciclo e o papel de goals, experiments e prompt_optimizer no aprendizado.
- Links para roadmap e Phase 3.

**Labels sugeridas:** `documentation`, `autonomy`

---

## Ordem sugerida

| Ordem | Issue | Motivo |
|-------|--------|--------|
| 1 | A1 (Dados reais no metrics/goals) | Base para A/B e otimização. |
| 2 | A2 (A/B e prompt_optimizer com dados reais) | Aprendizado com dados. |
| 3 | A3 (Core autonomous no daemon) | Contexto RAG no loop. |
| 4 | A4 (Doc feedback loop) | Clareza para evolução. |

---

**Última atualização:** 2026-03-11
