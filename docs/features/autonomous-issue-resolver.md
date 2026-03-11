# Autonomous Issue Resolver

**Data**: 2026-03-10
**Status**: ✅ Implementado (Under Development)
**Módulo**: `jarvis-rs/core/src/issue_resolver/`

## Visão Geral

O Autonomous Issue Resolver é um pipeline multi-estágio que resolve automaticamente issues do GitHub. O sistema analisa issues usando LLM, cria planos de implementação, executa as mudanças e cria pull requests.

## Motivação

- **Automação**: Reduzir trabalho manual na resolução de issues rotineiras
- **Consistência**: Aplicar o mesmo processo de análise e implementação
- **Velocidade**: Processar múltiplas issues em paralelo ou sequencialmente
- **Integração**: Integrar com o fluxo existente do Jarvis (tools GitHub, shell, etc.)

## Arquitetura

### Pipeline de Resolução

```
┌─────────────┐   ┌─────────────┐   ┌─────────────┐   ┌─────────────┐   ┌─────────────┐
│   Scanner   │──▶│   Context   │──▶│   Analyzer  │──▶│   Planner   │──▶│ Safety Gate │
│ (poll issues)│   │ (repo info) │   │ (LLM anal.) │   │ (LLM plan)  │   │ (classifier)│
└─────────────┘   └─────────────┘   └─────────────┘   └─────────────┘   └─────────────┘
                                                                                │
                                                                                ▼
                                                                         ┌─────────────┐
                                                                         │  Executor   │
                                                                         │ (apply fix) │
                                                                         └─────────────┘
```

### Componentes

| Componente | Descrição |
|------------|-----------|
| **Scanner** | Poll repositórios por issues com labels configuradas (ex: `jarvis-auto`) |
| **Context** | Coleta estrutura do repositório, linguagens e arquivos relevantes |
| **Analyzer** | Usa sub-agente LLM para produzir análise estruturada da issue |
| **Planner** | Usa sub-agente LLM para produzir plano de implementação |
| **Safety Gate** | Avalia o plano contra o classificador de segurança |
| **Executor** | Executa o plano (edits, shell, etc.), faz commit+push e cria o PR via API do GitHub (título e corpo do plano); preenche `pr_url` no resultado |

## Comandos CLI

### Exec (jarvis-exec)

```bash
# Resolver issue específica
jarvis resolve owner/repo --issue 42

# Escanear e resolver issues com labels (ex: jarvis-auto)
jarvis resolve owner/repo
```

**Requisitos**:
- **Token GitHub**: a mesma fonte usada pelas ferramentas GitHub do Agent. Ordem de resolução: (1) variável de ambiente `GITHUB_PAT` ou `jarvis_GITHUB_PAT`; (2) secrets do Jarvis, ex.: `jarvis secrets set GITHUB_PAT <token>`. O nome do secret pode ser configurado em `[github] pat_secret_name` no `config.toml` (default: `GITHUB_PAT`). Opcionalmente, `GITHUB_API_BASE_URL` ou `jarvis_GITHUB_API_BASE_URL` (ou `[github] api_base_url`) para GitHub Enterprise.
- Feature `autonomous_issue_resolver` habilitada (habilitada automaticamente ao usar `resolve`)
- Formato do repositório: `owner/repo`

### Configuração

```toml
# config.toml
[features]
autonomous_issue_resolver = true
```

Ou via CLI: `--enable autonomous_issue_resolver` ou `-c features.autonomous_issue_resolver=true`

## Especificação Técnica

### Feature Flag

- **Key**: `autonomous_issue_resolver`
- **Stage**: UnderDevelopment
- **Default**: `false`

### Protocolo

```rust
// IssueResolverRequest (protocol)
pub struct IssueResolverRequest {
    pub owner: String,
    pub repo: String,
    pub issue_number: Option<u64>,  // None = scan mode
}
```

### Scanner Config

- **required_labels**: Labels obrigatórias (default: `["jarvis-auto"]`)
- **exclude_labels**: Labels que excluem (default: `["wontfix", "in-progress"]`)
- **max_issues_per_scan**: Máximo por ciclo (default: 5)

## Fluxo de Dados

1. **CLI/Op**: Usuário executa `jarvis resolve owner/repo` ou envia `Op::Resolve`
2. **Resolução**: `resolve_issue_resolver_request` valida e monta prompt
3. **Spawn**: `spawn_issue_resolver_thread` cria thread com `IssueResolverTask`
4. **Task**: `IssueResolverTask` orquestra Scanner → Context → Analyzer → Planner → Safety Gate → Executor
5. **Output**: branch criada, commit+push, PR criado via API (com `pr_title` e `pr_body` do plano); eventos `EnteredIssueResolverMode` / `IssueResolverOutputEvent`; o resultado da execução inclui `pr_url` quando o PR for criado com sucesso

## Eventos de Rollout

- `EnteredIssueResolverMode(IssueResolverRequest)` — persistido em rollout
- `ExitedReviewMode` — quando sai do modo

## Referências

- [GitHub Integration](./github-integration.md)
- [CLI Autonomous Commands](./cli-autonomous-commands.md)
- [AUTONOMY_IMPLEMENTATION_STATUS.md](../AUTONOMY_IMPLEMENTATION_STATUS.md)

---

**Última atualização**: 2026-03-11
