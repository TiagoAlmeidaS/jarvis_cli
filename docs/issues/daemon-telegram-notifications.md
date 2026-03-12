# Issue sugerida: Notificações Telegram no daemon

Objetivo: documentar e facilitar a configuração das notificações Telegram do daemon (resumo diário, alertas de falhas e propostas pendentes).

Use este doc para criar a issue no repositório (GitHub/GitLab). O daemon já suporta Telegram via variáveis de ambiente; a issue cobre documentação e, opcionalmente, melhorias.

---

## Issue — Documentar e configurar notificações Telegram do daemon

**Título:** `docs(daemon): documentar configuração de notificações Telegram (Jarvis_Ai_pro_bot)`

**Descrição:**

O daemon já envia notificações via Telegram quando `JARVIS_TELEGRAM_BOT_TOKEN` e `JARVIS_TELEGRAM_CHAT_ID` estão definidos (ver `jarvis-rs/daemon/src/notifications.rs`): resumo diário no horário configurável (`JARVIS_NOTIFY_HOUR`, padrão 8), alertas de falhas e propostas pendentes. Atualmente isso só aparece no Prerequisites do DAEMON_QUICK_START como opcional; falta um passo a passo claro.

- Documentar em **RUNBOOK-DAEMON-GOOGLE.md** (ou em **deploy-servidor-casa.md** / **DAEMON_QUICK_START**) a seção "Notificações Telegram": como obter o Chat ID (enviar mensagem ao bot e usar getUpdates ou @userinfobot), onde definir as variáveis (`.env` no servidor ou no `docker-compose.homeserver.yml` como `environment`), e que o daemon deve ser reiniciado após alterar o `.env`.
- Garantir que **jarvis-rs/daemon/env.example** (e, se existir, `.env.example` na raiz) contenha os comentários para `JARVIS_TELEGRAM_BOT_TOKEN`, `JARVIS_TELEGRAM_CHAT_ID` e `JARVIS_NOTIFY_HOUR`.
- Opcional: mencionar o nome do bot de exemplo (ex.: Jarvis_Ai_pro_bot) na doc, sem expor o token (token só em `.env`, nunca versionado).

**Critério de aceite:**

- Existe seção ou runbook que descreve como configurar Telegram para o daemon (obter Chat ID, definir variáveis, reiniciar).
- `.env.example` ou `env.example` do daemon documenta as três variáveis Telegram.
- Nenhum token real em arquivos versionados.

**Labels sugeridas:** `documentation`, `daemon`

**Referências:** [DAEMON_QUICK_START](../DAEMON_QUICK_START.md), [jarvis-rs/daemon/src/notifications.rs](../../jarvis-rs/daemon/src/notifications.rs), [jarvis-rs/daemon/env.example](../../jarvis-rs/daemon/env.example).

---

**Última atualização:** 2026-03-12
