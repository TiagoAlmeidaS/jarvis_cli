# Issue: Testes de integração — fluxo do daemon com Google (Gemini)

Objetivo: ter **testes de integração** que validem o fluxo do daemon usando apenas o provedor Google (Gemini / free tier), desde a subida do daemon até a execução de pelo menos um pipeline.

Use este doc para criar a issue no seu repositório (GitHub/GitLab).

---

## Issue — Testes de integração do daemon com Google

**Título:** `test(daemon): adicionar testes de integração do fluxo daemon com Google (Gemini)`

**Descrição:**

Implementar testes de integração que cubram o fluxo do daemon quando configurado **somente** com `GOOGLE_API_KEY` (Google AI Studio / Gemini), para garantir que:

1. O daemon sobe com sucesso com env contendo apenas `GOOGLE_API_KEY` (e sem `OPENROUTER_API_KEY`).
2. Um pipeline configurado com `provider: "google"` e `model: "gemini-2.0-flash"` é executado com sucesso (pelo menos uma etapa que chame a API Gemini).
3. O fluxo não quebra quando OpenRouter não está configurado.

**Escopo sugerido:**

- **Cenário 1 (subida):** Teste que inicia o daemon com `GOOGLE_API_KEY` definida e (opcional) sem `OPENROUTER_API_KEY`; verifica que o processo sobe e o scheduler está ativo (ex.: healthcheck ou log esperado).
- **Cenário 2 (pipeline Google):** Teste que registra um pipeline com `provider: "google"`, `model: "gemini-2.0-flash"`, dispara uma execução (ex.: job manual ou tick) e verifica que a chamada ao LLM ocorre e retorna resposta (pode usar um pipeline leve, ex.: `strategy_analyzer` com mocks de métricas, ou `seo_blog` em modo que não publique).
- **Opcional:** Se a API Gemini não puder ser chamada em CI (rate limit, rede), documentar um teste manual com os mesmos critérios e/ou usar mock do endpoint Gemini para o teste automatizado.

**Onde implementar:**

- Se já existir suíte de testes do daemon (ex.: `jarvis-rs/daemon/tests/`), adicionar um módulo ou arquivo dedicado (ex.: `integration_google.rs` ou `e2e_google_flow.rs`).
- Se não existir, criar um teste de integração mínimo (ex.: script ou `#[test]` que sobe o daemon, injeta pipeline Google e asserta execução), ou documentar um **runbook de teste manual** em `docs/` com os passos e o resultado esperado.

**Critério de aceite:**

- Existe pelo menos um teste de integração (ou runbook de teste manual documentado) que valida o fluxo daemon + Google.
- O teste (ou runbook) está descrito em docs ou no código e pode ser reproduzido (ex.: `cargo test -p jarvis-daemon --test integration_google` ou seção em DAEMON_QUICK_START / deploy-servidor-casa).
- Nenhum regresso: o daemon continua funcionando com OpenRouter quando configurado.

**Labels sugeridas:** `testing`, `daemon`, `integration`

**Relacionado:** Conjunto [daemon-google-free-tier.md](daemon-google-free-tier.md) (Issue 6 — validar daemon só com Google). Esta issue detalha especificamente os **testes de integração** desse fluxo.

---

**Última atualização:** 2026-03-11
