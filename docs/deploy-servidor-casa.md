# Deploy no servidor em casa (Docker Compose) — OpenRouter, LLM de baixo custo

Guia para rodar o Jarvis daemon no seu servidor doméstico com Docker Compose, usando **OpenRouter** com modelos de **baixo custo** para manter o gasto com LLM sob controle.

---

## Por que servidor em casa + OpenRouter?

- **Uptime**: O daemon roda 24/7 no seu servidor, sem depender do PC.
- **Custo previsível**: OpenRouter com modelos baratos (ex.: Mistral Nemo, Llama 3.2) reduz o custo por token; você controla o uso pelos créditos da sua conta.
- **Simples**: Um único container (daemon); não precisa subir Ollama nem outros serviços.

---

## Pré-requisitos no servidor

- **Docker** e **Docker Compose** (v2)
- Arquivo **`.env`** na raiz do projeto com **`OPENROUTER_API_KEY`** (obtenha em [openrouter.ai](https://openrouter.ai))

```bash
cp .env.example .env
# Edite .env e defina:
# OPENROUTER_API_KEY=sk-or-v1-...
```

---

## Modelos OpenRouter de baixo custo

Para manter custo baixo, use no pipeline um modelo barato. Exemplos (verifique preços atuais em [openrouter.ai/models](https://openrouter.ai/models)):

| Modelo (OpenRouter) | Uso típico | Custo aproximado |
|--------------------|------------|------------------|
| `mistralai/mistral-nemo` | Conteúdo, análise, multilíngue | ~$0.02–0.04 / 1M tokens |
| `meta-llama/llama-3.2-3b-instruct` | Tarefas leves | Muito baixo |
| `google/gemma-2-9b-it` | Conteúdo e resumos | Baixo |
| `nousresearch/hermes-3-llama-3.1-8b` | Equilíbrio custo/qualidade | Baixo |

No JSON do pipeline use `"provider": "openrouter"` e `"model": "mistralai/mistral-nemo"` (ou outro id da tabela acima).

---

## Passo a passo

### 1. No servidor em casa

Copie o projeto para o servidor (ou clone do Git) **incluindo** `docker-compose.homeserver.yml` e `.env`:

```bash
# Exemplo: clone no servidor
git clone <seu-repo> jarvis_cli && cd jarvis_cli
cp .env.example .env
# Edite .env e defina OPENROUTER_API_KEY
```

### 2. Subir o daemon

```bash
cd jarvis_cli
docker compose -f docker-compose.homeserver.yml up -d
```

Isso sobe apenas o **jarvis-daemon**. O daemon lê `OPENROUTER_API_KEY` do `.env` e usa OpenRouter nos pipelines que estiverem configurados com `provider: "openrouter"`.

### 3. Configurar pipeline com OpenRouter (baixo custo)

Exemplo de pipeline SEO usando OpenRouter com modelo barato:

```json
{
  "id": "seo-blog-openrouter",
  "name": "SEO Blog (OpenRouter baixo custo)",
  "strategy": "seo_blog",
  "schedule_cron": "0 */6 * * *",
  "max_retries": 2,
  "llm": {
    "provider": "openrouter",
    "model": "mistralai/mistral-nemo",
    "temperature": 0.8,
    "max_tokens": 4000
  },
  "content": {
    "language": "pt-BR",
    "niche": "Tecnologia",
    "min_words": 800,
    "max_words": 1500
  }
}
```

Adicione e ative o pipeline via CLI (no seu PC ou onde tiver o Jarvis CLI apontando para o mesmo ambiente/DB, se aplicável):

```bash
jarvis daemon pipeline add seo-pipeline-openrouter.json
jarvis daemon pipeline enable seo-blog-openrouter
```

### 4. Verificar

```bash
docker compose -f docker-compose.homeserver.yml ps
docker logs -f jarvis-daemon
```

---

## Recursos sugeridos no servidor

- **RAM**: 512 MB são suficientes para o daemon (o LLM roda na nuvem via OpenRouter).
- **Rede**: estável para chamadas à API OpenRouter.
- **Disco**: espaço para imagens Docker e volume do daemon (SQLite, logs).

---

## Alternativas: Google (free tier) ou Ollama (local)

O mesmo `docker-compose.homeserver.yml` e o mesmo daemon servem para OpenRouter, Google ou Ollama; basta trocar o `.env` e a config do pipeline.

### Como usar Google (já implementado)

O daemon já suporta o provedor **Google** (Gemini). Cota gratuita generosa no [Google AI Studio](https://aistudio.google.com/).

1. **Chave de API**: em [Google AI Studio](https://aistudio.google.com/) → “Get API key” (ou use uma chave do Gemini API).  
2. **No `.env`** (no servidor, na raiz do projeto):
   ```bash
   GOOGLE_API_KEY=suachave
   ```
   O daemon aceita também `GEMINI_API_KEY` (mesma chave).  
3. **Pipeline de exemplo só Google:** use o arquivo pronto [docs/examples/daemon-pipeline-google.json](examples/daemon-pipeline-google.json) (ou `jarvis-rs/daemon/examples/pipeline-google-free-tier.json`). Já vem com `"provider": "google"` e `"model": "gemini-2.0-flash"`. Adicione e ative:
   ```bash
   jarvis daemon pipeline add docs/examples/daemon-pipeline-google.json
   jarvis daemon pipeline enable seo-google-free
   ```
4. **Compose**: nenhuma mudança. Sobe com o mesmo comando:
   ```bash
   docker compose -f docker-compose.homeserver.yml up -d
   ```
   O daemon lê `GOOGLE_API_KEY` e usa a API Gemini (endpoint OpenAI-compatible da Google).

Assim você reduz ou elimina o uso de créditos OpenRouter usando o free tier do Google.

### Como usar Ollama (local)

Se quiser **zero** custo de API e tiver RAM no servidor para rodar modelo local:

- Use `docker-compose.vps.yml` (inclui daemon + Ollama) ou adicione um serviço Ollama ao seu compose.
- No pipeline: `"provider": "ollama"`, `"base_url": "http://ollama:11434/v1"`, `"model": "llama3.2"`.

Para mais opções, veja [daemon-deploy-alternatives.md](architecture/daemon-deploy-alternatives.md).

---

## Referências

- [DAEMON_QUICK_START.md](DAEMON_QUICK_START.md) — Comandos do daemon, pipelines, goals.
- [Runbook: daemon somente com Google](RUNBOOK-DAEMON-GOOGLE.md) — Checklist de uma página para subir o daemon só com Google (free tier).
- [Pasta de issues](issues/README.md) — Issues centralizadas; [daemon-google-free-tier](issues/daemon-google-free-tier.md) para o daemon só com Google (free tier).
- [daemon-deploy-alternatives.md](architecture/daemon-deploy-alternatives.md) — Ollama, Google, outros provedores.
- [docker-compose.homeserver.yml](../docker-compose.homeserver.yml) — Compose para servidor em casa (OpenRouter, LLM de baixo custo).
- [OpenRouter Models](https://openrouter.ai/models) — Listagem e preços de modelos.

---

**Última atualização**: 2026-03-11
