# Alternativas para rodar o daemon (sem depender de créditos OpenRouter)

**Objetivo**: Permitir colocar o daemon Jarvis em execução mesmo com limitação de créditos no OpenRouter (ou outro provedor pago). Este doc lista opções de provedores, modelos locais e estratégias de redução de custo.

---

## Situação atual

- O daemon está **implementado no código** (scheduler, pipelines, goals, executor, etc.).
- Os pipelines usam **LLM** (geração de conteúdo, strategy analyzer, etc.) e hoje a doc e exemplos assumem **OpenRouter** (ou similar) com API key.
- **Limitação**: créditos OpenRouter insuficientes impedem rodar o daemon em uso contínuo.

---

## 1. Provedores com free tier ou custo baixo

| Provedor | Free tier / custo | Como usar no daemon |
|----------|-------------------|---------------------|
| **Google AI (Gemini)** | Cota gratuita generosa (Gemini 1.5 Flash, etc.) | Configurar `GOOGLE_API_KEY`; no pipeline `llm.provider` usar provedor compatível com Google (se o daemon suportar). Ver [DAEMON_QUICK_START.md](../DAEMON_QUICK_START.md) — já menciona `GOOGLE_API_KEY` como alternativa. |
| **Groq** | Free tier com rate limit | API compatível OpenAI; configurar base URL e key; modelos muito rápidos (Llama, etc.). |
| **Together.ai** | Créditos iniciais / preço baixo | OpenAI-compatible API; vários modelos open source. |
| **Ollama (local)** | Sem custo de API (roda na sua máquina) | Instalar Ollama, subir um modelo (ex.: Llama, Mistral). Se o daemon tiver cliente Ollama ou endpoint OpenAI-compatible local, apontar para `http://localhost:11434`. |
| **LM Studio (local)** | Sem custo (local) | Servidor local OpenAI-compatible; rodar modelo na máquina e apontar o daemon para a URL local. |
| **OpenAI** | Pago (pode ter créditos iniciais) | `OPENAI_API_KEY`; usar modelos mais baratos (gpt-4o-mini) para reduzir custo. |

**Recomendação**: Priorizar **Google API (Gemini free tier)** ou **Ollama/local** para não depender de créditos OpenRouter. Verificar no código do daemon (`jarvis-rs/daemon`, `processor/router.rs`, config de pipeline) quais `provider` e variáveis de ambiente são suportados.

---

## 2. Reduzir consumo de créditos (quando usar OpenRouter ou pago)

- **Cron menos frequente**: Aumentar intervalo dos pipelines (ex.: SEO a cada 12 h em vez de 4 h).
- **Modelos mais baratos**: Usar no pipeline um modelo menor/mais barato (ex.: `mistralai/mistral-nemo` já é econômico; verificar listagem OpenRouter por preço).
- **Menos pipelines ativos**: Habilitar só 1–2 pipelines (ex.: só metrics_collector + strategy_analyzer, sem SEO blog por enquanto).
- **Limite de tokens**: Reduzir `max_tokens` no config do pipeline para diminuir custo por chamada.

---

## 3. Modelos locais (Ollama / LM Studio)

Se o daemon suportar **endpoint OpenAI-compatible**:

1. **Ollama**: `ollama run <modelo>` (ex.: `llama3.2`, `mistral`). Por padrão expõe `http://localhost:11434/v1` compatível com OpenAI.
2. **LM Studio**: Abrir um modelo e “Start Server”; usar a URL indicada (ex.: `http://localhost:1234/v1`) e, se necessário, API key vazia ou fixa.
3. No config do pipeline do daemon, definir `provider` e `base_url` (e `api_key` se exigido) para esse endpoint.

**Vantagem**: Custo zero de API; limite é só CPU/RAM da máquina. **Desvantagem**: Qualidade e velocidade dependem do hardware; modelos menores podem gerar texto pior que cloud.

---

## 4. Próximos passos no projeto

1. **Confirmar no código** quais provedores o daemon aceita (OpenRouter, Google, OpenAI, URL genérica?) e quais variáveis de ambiente cada um usa.
2. **Documentar** em [DAEMON_QUICK_START.md](../DAEMON_QUICK_START.md) um “Modo mínimo” com Google free tier ou Ollama, passo a passo.
3. **Testar** um único pipeline (ex.: metrics_collector, que pode não precisar de LLM em todo run) para validar que o daemon sobe e persiste no SQLite; em seguida ativar um pipeline que use LLM com provedor alternativo.

---

## 5. Referências

- [DAEMON_QUICK_START.md](../DAEMON_QUICK_START.md) — Pré-requisitos (OPENROUTER_API_KEY ou GOOGLE_API_KEY), Docker e execução local.
- [Deploy no servidor em casa](../deploy-servidor-casa.md) — Rodar daemon no servidor em casa com Docker Compose e OpenRouter (LLM de baixo custo) (`docker-compose.homeserver.yml`).
- [autonomy-roadmap.md](./autonomy-roadmap.md) — Estado do daemon (implementado vs em deploy) e roadmap.
- Código: `jarvis-rs/daemon/src/processor/` (router, cliente LLM), config de pipeline (`llm.provider`, `llm.model`).

---

**Última atualização**: 2026-03-11
