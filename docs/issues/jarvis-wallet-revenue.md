# Issue sugerida: Carteira do Jarvis (fundo de receita)

Objetivo: ter um **lugar (conceitual ou implementado)** onde a receita gerada pelo Jarvis (ex.: AdSense de sites gerenciados, comissões de afiliados) seja atribuída e, se desejado, acumulada — a "carteira" do próprio sistema.

Use este doc para criar a issue no repositório ou para registrar a direção de produto. **Não implementar agora;** apenas documentar a necessidade e as opções.

---

## Contexto

Quando o daemon/Jarvis gera receita (ex.: anúncios em blogs que ele alimenta, comissões), surge a pergunta: **onde esse fundo fica?** A ideia é o sistema ter sua própria "carteira" para essa receita, em vez de ficar apenas em contas externas sem atribuição clara ao Jarvis.

---

## Escopo possível (futuro)

| Opção | Descrição |
|-------|-----------|
| **Ledger interno** | Tabela/saldo no banco do daemon: "receita atribuída" por fonte (AdSense, afiliado, etc.) e data. Sem movimentação real de dinheiro; apenas contabilidade interna. |
| **Relatório / dashboard** | Visibilidade: relatório "receita do Jarvis" agregada por pipeline/site/período, sem bloquear execução. |
| **Integração com meios de pagamento** | Futuro: integrar com carteira externa ou meio de pagamento para acumular ou retirar o saldo (fora do escopo imediato). |

---

## Issue sugerida (quando for priorizar)

**Título (exemplo):** `feat(daemon): carteira do Jarvis — ledger de receita atribuída`

**Objetivo:** Registrar e exibir a receita gerada pelo Jarvis (atribuída por fonte e data), como primeiro passo para uma "carteira" do sistema.

**Descrição:** O daemon já pode coletar métricas de receita (ex.: AdSense, WordPress) via data sources. Falta um conceito de "carteira" ou ledger onde essa receita seja atribuída ao Jarvis (por pipeline, site, período). Implementar tabela(s) e API ou relatório que permitam: (1) registrar receita atribuída quando os data sources reportarem; (2) consultar saldo ou histórico (dashboard ou CLI).

**Implementação / Escopo (sugestão):**

- Definir modelo de dados (ex.: `revenue_entries`: source_type, pipeline_id, amount, currency, recorded_at).
- Inserir entradas quando os data sources (AdSense, etc.) retornarem dados de receita.
- Expor consulta: saldo agregado e/ou histórico (endpoint ou `jarvis daemon wallet` / relatório em docs).

**Critério de aceite:**

- Existe armazenamento (DB) de receita atribuída ao Jarvis por fonte e data.
- É possível consultar totais ou histórico (CLI, dashboard ou doc).

**Referências:** [board-renda-phases.md](board-renda-phases.md) (Fase 4 — porta para renda), [evolucao-board-e-renda-levantamento.md](../architecture/evolucao-board-e-renda-levantamento.md).

**Labels sugeridas:** `feature`, `daemon`, `board`, `revenue`

---

**Última atualização:** 2026-03-12
