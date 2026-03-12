# Issues sugeridas: Daemon funcionando somente com Google (free tier)

Objetivo: ter o daemon rodando **por completo** usando **apenas** a versão free do Google (Gemini), para poder subir no servidor em casa e aplicar as atividades (pipelines) sem depender de OpenRouter.

Use este doc para criar as issues no seu repositório (GitHub/GitLab). Cada bloco abaixo é uma issue pronta para copiar.

---

## Issue 1 — Docs: "Modo mínimo Google free tier" no Quick Start

**Título:** `docs: documentar modo mínimo do daemon com só Google (free tier)`

**Descrição:**

Garantir que um usuário consiga subir o daemon **sem** OpenRouter, usando apenas `GOOGLE_API_KEY` (Google AI Studio / Gemini free tier).

- Em **DAEMON_QUICK_START.md**:
  - Na seção "Prerequisites", deixar explícito que o **mínimo** pode ser só `GOOGLE_API_KEY` (e que OpenRouter é opcional nesse modo).
  - Adicionar um sub-bloco ou nota: "Modo mínimo (só Google): defina apenas `GOOGLE_API_KEY` no `.env` e use pipelines com `provider: \"google\"`."
- Opcional: em **deploy-servidor-casa.md**, no início ou em "Pré-requisitos", mencionar que para usar **somente** Google não é necessário `OPENROUTER_API_KEY`.

**Critério de aceite:**

- Leitura do Quick Start deixa claro que o daemon pode rodar só com Google.
- Deploy no servidor em casa (doc) indica que, no modo Google, só `GOOGLE_API_KEY` é obrigatório.

**Labels sugeridas:** `documentation`, `daemon`

---

## Issue 2 — .env.example: GOOGLE_API_KEY como opção mínima

**Título:** `chore: documentar GOOGLE_API_KEY no .env.example para modo Google-only`

**Descrição:**

Garantir que o `.env.example` na raiz do projeto (e, se houver, em `jarvis-rs/`) documente o modo "só Google":

- Incluir comentário para `GOOGLE_API_KEY` (e, se aplicável, `GEMINI_API_KEY`) com texto do tipo: "Obrigatório para pipelines com provider google; suficiente para rodar o daemon sem OpenRouter."
- Manter `OPENROUTER_API_KEY` como opcional quando o foco for Google (ex.: comentário "Opcional se usar apenas provider google").

**Critério de aceite:**

- `.env.example` contém `GOOGLE_API_KEY` com comentário explicativo.
- Fica claro que é possível rodar o daemon apenas com Google.

**Labels sugeridas:** `chore`, `daemon`

---

## Issue 3 — Pipeline de exemplo com Google (free tier)

**Título:** `feat(daemon): adicionar pipeline de exemplo usando só Google (gemini-2.0-flash)`

**Descrição:**

Incluir no repositório **um** arquivo de pipeline pronto que use apenas o provedor Google (free tier), para o usuário poder testar o daemon sem OpenRouter.

- Criar um JSON de pipeline (ex.: `seo_blog` ou `strategy_analyzer`) com:
  - `"provider": "google"`
  - `"model": "gemini-2.0-flash"`
- Colocar em um local óbvio, por exemplo:
  - `jarvis-rs/daemon/examples/pipeline-google-free-tier.json`, ou
  - `docs/examples/daemon-pipeline-google.json`
- Referenciar esse arquivo em **DAEMON_QUICK_START.md** e/ou **deploy-servidor-casa.md** (ex.: "Pipeline de exemplo só Google: `docs/examples/daemon-pipeline-google.json`").

**Critério de aceite:**

- Existe um pipeline JSON que usa só `provider: "google"` e `model: "gemini-2.0-flash"`.
- Documentação indica onde está o exemplo e como usá-lo (`jarvis daemon pipeline add ...`).

**Labels sugeridas:** `feature`, `daemon`

---

## Issue 4 — Docker Compose: documentar uso só com GOOGLE_API_KEY

**Título:** `docs(docker): deixar claro que homeserver pode rodar só com GOOGLE_API_KEY`

**Descrição:**

Garantir que o uso do `docker-compose.homeserver.yml` com **apenas** Google (sem OpenRouter) esteja explícito.

- No cabeçalho ou comentários de **docker-compose.homeserver.yml**: mencionar que `OPENROUTER_API_KEY` não é obrigatório se todos os pipelines usarem `provider: "google"`; o mínimo é `GOOGLE_API_KEY`.
- Em **deploy-servidor-casa.md**: na seção de pré-requisitos ou no passo a passo, incluir uma variante "Só Google (free tier)" com:
  - `.env` só com `GOOGLE_API_KEY`
  - Comando `docker compose -f docker-compose.homeserver.yml up -d`
  - Uso do pipeline de exemplo com Google (ver Issue 3).

**Critério de aceite:**

- Quem seguir a doc consegue subir o daemon no servidor em casa usando apenas `GOOGLE_API_KEY` e um pipeline Google.
- Comentários do compose não exigem OpenRouter para esse modo.

**Labels sugeridas:** `documentation`, `daemon`, `docker`

---

## Issue 5 — Runbook: "Subir daemon só com Google" (checklist)

**Título:** `docs: runbook para subir o daemon somente com Google (free tier)`

**Descrição:**

Criar um **checklist/runbook** de uma página que permita a qualquer pessoa subir o daemon e tê-lo "funcionando por completo" usando só Google (free tier).

Conteúdo sugerido (pode ser uma seção em **deploy-servidor-casa.md** ou um **RUNBOOK-DAEMON-GOOGLE.md**):

1. Pré-requisitos: Docker + Docker Compose, conta Google AI Studio, `GOOGLE_API_KEY`.
2. Clone/cópia do repo no servidor (ou máquina local).
3. `.env`: definir apenas `GOOGLE_API_KEY`.
4. `docker compose -f docker-compose.homeserver.yml up -d`.
5. Adicionar e habilitar pelo menos um pipeline que use `provider: "google"` (ex.: o da Issue 3).
6. Verificação: `docker logs -f jarvis-daemon` e, se possível, confirmação de um job executado (ex.: metrics_collector ou seo com draft no WordPress).
7. (Opcional) Limites do free tier: referência a rate limits/cota do Gemini e sugestão de cron menos agressivo se necessário.

**Critério de aceite:**

- Existe um runbook/checklist seguindo qual alguém consegue subir e ver o daemon executando atividades com só Google.
- Doc referenciada no índice (ex.: README da pasta docs ou DAEMON_QUICK_START).

**Labels sugeridas:** `documentation`, `daemon`

---

## Issue 6 — Validar daemon sem OPENROUTER (só Google)

**Título:** `test(daemon): validar que o daemon sobe e executa pipeline só com GOOGLE_API_KEY`

**Descrição:**

Validar (manual ou automatizado) que o daemon:

1. Sobe com sucesso quando no ambiente há **apenas** `GOOGLE_API_KEY` (e **sem** `OPENROUTER_API_KEY`).
2. Consegue executar pelo menos um pipeline configurado com `provider: "google"` e `model: "gemini-2.0-flash"` (ex.: strategy_analyzer ou seo_blog em modo draft).

Opções:

- **Manual**: descrever no README ou em docs os passos de teste e o resultado esperado (ex.: "Rodar com .env só GOOGLE_API_KEY; adicionar pipeline Google; verificar log de execução").
- **Automatizado**: se houver testes E2E do daemon, adicionar um cenário (ou CI job) que rode o daemon com env só Google e um pipeline Google (pode ser mock da API Gemini se for mais simples).

**Critério de aceite:**

- Fica documentado ou automatizado que o "modo só Google" funciona.
- Nenhum regresso: daemon continua funcionando com OpenRouter quando configurado.

**Labels sugeridas:** `testing`, `daemon`

---

## Ordem sugerida para implementar

| Ordem | Issue | Motivo |
|-------|--------|--------|
| 1 | Issue 2 (.env.example) | Rápido; deixa o mínimo claro para quem for testar. |
| 2 | Issue 3 (pipeline exemplo Google) | Entrega o artefato que as outras docs vão referenciar. |
| 3 | Issue 1 (Quick Start modo Google) | Docs alinhadas com o modo mínimo. |
| 4 | Issue 4 (Docker só Google) | Deixa o deploy no servidor em casa explícito. |
| 5 | Issue 5 (runbook) | Consolida tudo em um checklist único. |
| 6 | Issue 6 (validar só Google) | Garante que o fluxo funciona e não quebra. |

---

## Resumo

- **Objetivo:** daemon funcionando por completo usando **somente** Google (free tier).
- **Entregas:** documentação (modo mínimo, runbook, Docker), `.env.example`, pipeline de exemplo Google, e validação (manual ou teste).
- **Resultado:** qualquer pessoa (ou você no servidor em casa) consegue subir o daemon e aplicar atividades (pipelines) sem ter OpenRouter configurado.

**Última atualização:** 2026-03-11
