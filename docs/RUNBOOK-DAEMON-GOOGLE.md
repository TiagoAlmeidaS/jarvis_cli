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

### 8. (Opcional) Notificações Telegram

O daemon envia resumo diário e alertas (falhas, propostas pendentes) via Telegram quando as variáveis abaixo estiverem definidas no `.env` (ou no `environment` do container). **Nunca coloque o token em arquivos versionados** — use apenas em `.env` no servidor.

1. **Obter o Chat ID**  
   - **Método A (getUpdates):** Envie uma mensagem **nova** para o seu bot (ex.: "oi" ou `/start` em [@Jarvis_Ai_pro_bot](https://t.me/Jarvis_Ai_pro_bot)). **Sem fechar o Telegram**, abra no navegador (substitua `SEU_TOKEN` pelo token do BotFather):  
     `https://api.telegram.org/botSEU_TOKEN/getUpdates`  
     No JSON, em `result[0].message.chat.id` (ou `result[0].chat.id`) estará o número. Use esse valor como `JARVIS_TELEGRAM_CHAT_ID`.  
     Se aparecer `"result": []`, os updates já foram consumidos: envie **outra** mensagem ao bot e abra o link de novo **uma única vez** (cada chamada a getUpdates “consome” as mensagens).
   - **Método B (mais fácil):** Abra [@userinfobot](https://t.me/userinfobot), envie `/start`. O bot responde com o seu **Id** (número). Para chat privado com o seu bot, esse Id é o mesmo que `JARVIS_TELEGRAM_CHAT_ID`.

2. **Definir variáveis no `.env`** (no servidor, na pasta do projeto):

   ```env
   JARVIS_TELEGRAM_BOT_TOKEN=seu_token_do_botfather
   JARVIS_TELEGRAM_CHAT_ID=123456789
   JARVIS_NOTIFY_HOUR=8
   ```

   `JARVIS_NOTIFY_HOUR` (0–23) é a hora do resumo diário; padrão 8.

3. **Reiniciar o daemon** para carregar o novo `.env`:

   ```bash
   docker compose -f docker-compose.homeserver.yml down
   docker compose -f docker-compose.homeserver.yml up -d
   ```

4. **Conferir nos logs** — deve aparecer algo como:  
   `Telegram notifier active (chat_id=..., daily summary at 8:00)` em vez de "Telegram notifications not configured".

Documentação da issue: [daemon-telegram-notifications.md](issues/daemon-telegram-notifications.md).

### 9. Testes de integração (daemon + Google)

Para validar o fluxo daemon + Google sem depender de API real:

```bash
cd jarvis-rs
cargo test -p jarvis-daemon --test integration_google
```

- **`daemon_starts_with_google_api_key_only`** — sobe o binário com só `GOOGLE_API_KEY`, verifica o log "Jarvis Daemon started" e encerra.
- **`pipeline_google_executes_against_mock_gemini`** — usa WireMock como endpoint Gemini, registra pipeline com `provider: "google"` e `model: "gemini-2.0-flash"`, executa o pipeline SEO blog e valida o resultado.

Não é necessário `GOOGLE_API_KEY` real; o teste do pipeline usa mock. Ver [daemon-integration-tests-google.md](issues/daemon-integration-tests-google.md).

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
| Telegram   | (Opcional) `JARVIS_TELEGRAM_BOT_TOKEN`, `JARVIS_TELEGRAM_CHAT_ID`, `JARVIS_NOTIFY_HOUR` no `.env`; reiniciar daemon |

---

## Referências

- [DAEMON_QUICK_START.md](DAEMON_QUICK_START.md) — Comandos do daemon, pipelines, goals.
- [Deploy no servidor em casa](deploy-servidor-casa.md) — Deploy com OpenRouter; seção “Como usar Google” complementa este runbook.
- [daemon-google-free-tier.md](issues/daemon-google-free-tier.md) — Conjunto de issues para o modo só Google.
- [daemon-telegram-notifications.md](issues/daemon-telegram-notifications.md) — Issue/doc para notificações Telegram do daemon.
- [docker-compose.homeserver.yml](../docker-compose.homeserver.yml) — Compose usado neste runbook.

---

**Última atualização:** 2026-03-12
