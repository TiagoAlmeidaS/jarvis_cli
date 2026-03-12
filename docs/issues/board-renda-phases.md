# Issues sugeridas: Fases Board e Renda (4 fases)

Objetivo: implementar o caminho do board (GitHub Issues) configurável até o loop autônomo e a porta para renda, conforme o [levantamento Board e Renda](../architecture/evolucao-board-e-renda-levantamento.md). O daemon **orquestra**; o core **executa** (IssueResolverTask, Scanner, Scheduler, PipelineRegistry, daemon-common). Não criar segundo scanner nem segundo executor de issues.

Use este doc para criar as issues no seu repositório (GitHub/GitLab). Cada bloco abaixo é uma issue pronta para copiar.

---

## Fase 1 — Board configurável

### Issue F1a — Seção `[issue_resolver]` em config e schema

**Título:** `feat(config): adicionar seção [issue_resolver] em config.toml e schema`

**Descrição:**

Definir "o que é o board" em config: repositórios e critérios de issues (labels, limites). Fonte: [evolucao-board-e-renda-levantamento.md](../architecture/evolucao-board-e-renda-levantamento.md) Fase 1.

- Adicionar seção `[issue_resolver]` em config.toml com campos: repos (lista de owner/repo), required_labels, exclude_labels, max_issues_per_scan.
- Atualizar schema (config.schema.json) e tipos em core (ConfigToml); documentar em docs de config.
- Regra: daemon e core usam o mesmo config; não duplicar definição.

**Critério de aceite:**

- config.toml aceita `[issue_resolver]`; schema validado; doc descreve os campos.
- Se mudou ConfigToml ou nested config, rodar `just write-config-schema` e incluir alteração no PR.

**Labels sugeridas:** `feature`, `config`, `board`

---

### Issue F1b — Scanner e CLI `jarvis resolve` sem args leem config

**Título:** `feat(core,cli): Scanner e jarvis resolve sem args usam config [issue_resolver]`

**Descrição:**

Fazer o Scanner e o fluxo de resolução de issues lerem a seção `[issue_resolver]` do config. CLI: `jarvis resolve` sem argumentos usa repo(s) e labels definidos no config. Fonte: Fase 1 do [levantamento](../architecture/evolucao-board-e-renda-levantamento.md).

- Core: IssueScanner e do_resolve_issue (ou equivalente) leem repos, required_labels, exclude_labels, max_issues_per_scan do config em vez de só defaults no código.
- CLI (jarvis-rs/exec): `jarvis resolve` sem owner/repo invoca o issue resolver com parâmetros vindos do config.
- Documentar que "board = Issues" definido em config.

**Critério de aceite:**

- `jarvis resolve` sem argumentos usa repo(s) e labels do config; doc "board = Issues" atualizada.

**Labels sugeridas:** `feature`, `core`, `cli`, `board`

---

## Fase 2 — Loop autônomo

### Issue F2a — Job/pipeline no daemon que invoca issue resolver do core

**Título:** `feat(daemon): job/pipeline que invoca issue resolver do core (lê config, scan, pick first, resolve)`

**Descrição:**

Registrar um job ou pipeline no daemon que, em cada execução, invoca o issue resolver do core: lê config `[issue_resolver]`, faz scan, escolhe a primeira issue (ou até max_issues_per_scan), resolve. Fonte: Fase 2 do [levantamento](../architecture/evolucao-board-e-renda-levantamento.md). Não implementar segundo scanner no daemon; reutilizar IssueResolverTask / entrypoint do core.

- Daemon: novo pipeline ou job registrado no Scheduler/PipelineRegistry que chama o core (ex.: spawn_issue_resolver_thread ou binário exec com parâmetros derivados do config).
- Limite de issues por execução respeitado (max_issues_per_scan).
- Documentar "board autônomo" (quando o job roda, como configurar cron).

**Critério de aceite:**

- Job agendado no daemon executa o issue resolver do core; pelo menos 1 issue pode ser resolvida por execução quando houver issues elegíveis.
- Doc "board autônomo" descreve o fluxo.

**Labels sugeridas:** `feature`, `daemon`, `board`

---

### Issue F2b — Comentar e atualizar issue após execução (PR ou erro)

**Título:** `feat(core): comentar e atualizar issue após resolução (PR ou erro)`

**Descrição:**

Após cada execução do issue resolver (sucesso ou falha), comentar na issue com o resultado (link do PR ou mensagem de erro) e, opcionalmente, atualizar labels. Fonte: Fase 2 do [levantamento](../architecture/evolucao-board-e-renda-levantamento.md).

- Core (ou ferramentas GitHub): ao finalizar resolução, postar comentário na issue (ex.: "Resolvido via PR #X" ou "Falha: ...").
- Opcional: atualizar labels (ex.: add "resolved" ou "failed") para manter o board consistente.
- Garantir que o daemon job (F2a) beneficia dessa informação ao rodar em loop.

**Critério de aceite:**

- Issue recebe comentário após cada tentativa de resolução (PR ou erro); opcionalmente labels atualizados.
- Nenhum regresso quando CLI chama `jarvis resolve` manualmente.

**Labels sugeridas:** `feature`, `core`, `board`

---

## Fase 3 — Métricas e custo

### Issue F3 — Registrar custo por run e comando `jarvis board stats`

**Título:** `feat(daemon): registrar custo por run (tokens/tempo) e comando jarvis board stats`

**Descrição:**

Medir custo operacional por execução do issue resolver (tokens, tempo, sucesso/falha); persistir no daemon (SQLite) e expor via comando ou dashboard. Fonte: Fase 3 do [levantamento](../architecture/evolucao-board-e-renda-levantamento.md).

- Registrar por run: tokens/custo estimado, tempo, sucesso/falha; persistir em daemon-common (tabela ou API existente).
- Core ou daemon reporta métricas ao final de cada execução (quem invocou persiste).
- Novo comando ou subcomando: `jarvis board stats` (ou equivalente no daemon_cmd) para resumo (ex.: runs, custo total, última execução).
- Documentar métricas e como usar.

**Critério de aceite:**

- Custo por execução persistido e visível em CLI (ex.: `jarvis board stats`); doc descreve métricas.

**Labels sugeridas:** `feature`, `daemon`, `board`

---

## Fase 4 — Renda

### Issue F4 — Abstração "fonte de receita" e doc fluxo receita > custo

**Título:** `feat(daemon,docs): abstração fonte de receita e doc fluxo receita > custo`

**Descrição:**

Abrir porta para "Jarvis gerar renda": abstração "fonte de receita" (config ou tipo em daemon-common), documentar um fluxo de receita automatizado (ex.: SEO/AdSense) e a meta "receita > custo". Opcional: goal "receita mínima" no daemon. Fonte: Fase 4 do [levantamento](../architecture/evolucao-board-e-renda-levantamento.md).

- Abstração "fonte de receita" (ex.: tipo ou config) permitindo plugar AdSense, affiliate, Stripe, etc.
- Documentar um fluxo de receita (ex.: SEO → conteúdo → AdSense) e onde entra custo (board stats) para meta "receita > custo".
- Opcional: goal no daemon "receita mínima" usando dados já coletados (real-data-integration).

**Critério de aceite:**

- Um fluxo de receita documentado; meta "receita > custo" descrita; abstração permite plugar outras fontes depois.

**Labels sugeridas:** `feature`, `daemon`, `documentation`

---

## Ordem e dependências

| Ordem | Fase / Issue | Dependência |
|-------|----------------|-------------|
| 1 | F1a, F1b (Board configurável) | Nenhuma |
| 2 | F2a, F2b (Loop autônomo) | Fase 1 |
| 3 | F3 (Métricas e custo) | Fase 2 em andamento ou concluída |
| 4 | F4 (Renda) | Pode ser em paralelo à Fase 3 |

---

**Última atualização:** 2026-03-11
