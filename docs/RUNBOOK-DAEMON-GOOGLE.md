# Runbook: Subir o daemon somente com Google (free tier)

Checklist de uma página para ter o Jarvis daemon **funcionando por completo** usando apenas **Google AI (Gemini) free tier**, sem OpenRouter nem outras APIs pagas.

---

## Checklist

- [ ] **1. Pré-requisitos** — Docker + Docker Compose (v2); conta no [Google AI Studio](https://aistudio.google.com/) e chave de API (Get API key).
- [ ] **2. Clone/cópia do projeto** — No servidor (ou máquina local) com `docker-compose.homeserver.yml` e `.env.example` na raiz.
- [ ] **3. `.env` só com Google** — Criar/editar `.env` com **apenas** `GOOGLE_API_KEY=suachave`. Não é necessário `OPENROUTER_API_KEY` para este modo.
- [ ] **4. Subir o daemon** — `docker compose -f docker-compose.homeserver.yml up -d`.
- [ ] **5. Pipeline com provider Google** — Adicionar e habilitar pelo menos um pipeline que use `"provider": "google"` e `"model": "gemini-2.0-flash"` (ou outro modelo free tier). Ex.: `jarvis daemon pipeline add <arquivo>.json` e `jarvis daemon pipeline enable <id>`.
- [ ] **6. Verificação** — `docker logs -f jarvis-daemon`; confirmar que um job/pipeline executou (ex.: metrics_collector ou seo em modo draft).
- [ ] **7. (Opcional) Limites do free tier** — Consultar cota/rate limits do Gemini; se necessário, usar `schedule_cron` menos agressivo nos pipelines (ex.: a cada 6–12 h em vez de a cada hora).

---

## Detalhes por passo

### 1. Pré-requisitos

- **Docker** e **Docker Compose** v2 instalados.
- Conta no [Google AI Studio](https://aistudio.google.com/) e **API key** (Get API key). A cota gratuita do Gemini é suficiente para rodar o daemon em modo leve.

### 2. Clone/cópia

```bash
git clone <seu-repo> jarvis_cli && cd jarvis_cli
```

Ou copie a pasta do projeto para o servidor, garantindo que existam `docker-compose.homeserver.yml` e `.env.example`.

### 3. `.env` só com Google

```bash
cp .env.example .env
# Edite .env e deixe apenas (ou no mínimo):
# GOOGLE_API_KEY=sua_chave_do_google_ai_studio
```

O daemon aceita também `GEMINI_API_KEY` (mesma chave). Para este runbook, **não** defina `OPENROUTER_API_KEY`.

### 4. Subir o daemon

```bash
docker compose -f docker-compose.homeserver.yml up -d
```

Isso sobe apenas o container do daemon. Ele lê `GOOGLE_API_KEY` do ambiente e usará Google nos pipelines configurados com `provider: "google"`.

### 5. Adicionar pipeline Google

Use o pipeline de exemplo pronto (provider `google`, model `gemini-2.0-flash`):

- **[docs/examples/daemon-pipeline-google.json](examples/daemon-pipeline-google.json)** (na raiz do repo: `docs/examples/daemon-pipeline-google.json`)
- Ou **jarvis-rs/daemon/examples/pipeline-google-free-tier.json**

Adicione e ative (via CLI no mesmo host ou onde o Jarvis CLI aponte para o daemon):

```bash
jarvis daemon pipeline add docs/examples/daemon-pipeline-google.json
jarvis daemon pipeline enable seo-google-free
```

### 6. Verificação

```bash
docker compose -f docker-compose.homeserver.yml ps   # container “Up”
docker logs -f jarvis-daemon
```

Confirme nos logs que o scheduler está ativo e que pelo menos um job do pipeline foi executado (ex.: geração de draft ou coleta de métricas). Se houver erro de API, verifique a chave e a cota do Gemini.

### 7. (Opcional) Limites do free tier

O [Gemini free tier](https://ai.google.dev/pricing) tem limites de RPM/TPM. Se o daemon for interrompido por rate limit, aumente o intervalo do cron (ex.: `"0 */12 * * *"` para a cada 12 h) ou reduza a frequência dos pipelines mais pesados.

---

## Resumo

| Item        | Ação |
|------------|------|
| Pré-requisitos | Docker, Docker Compose, Google AI Studio + API key |
| Projeto    | Clone/cópia com `docker-compose.homeserver.yml` |
| `.env`     | Apenas `GOOGLE_API_KEY=...` |
| Daemon     | `docker compose -f docker-compose.homeserver.yml up -d` |
| Pipeline   | Pelo menos um com `provider: "google"`, `model: "gemini-2.0-flash"` |
| Verificação | `docker logs -f jarvis-daemon` e confirmação de job executado |
| Limites    | Ajustar cron se precisar respeitar cota free |

---

## Referências

- [DAEMON_QUICK_START.md](DAEMON_QUICK_START.md) — Comandos do daemon, pipelines, goals.
- [Deploy no servidor em casa](deploy-servidor-casa.md) — Deploy com OpenRouter; seção “Como usar Google” complementa este runbook.
- [daemon-google-free-tier.md](issues/daemon-google-free-tier.md) — Conjunto de issues para o modo só Google.
- [docker-compose.homeserver.yml](../docker-compose.homeserver.yml) — Compose usado neste runbook.

---

**Última atualização:** 2026-03-11
