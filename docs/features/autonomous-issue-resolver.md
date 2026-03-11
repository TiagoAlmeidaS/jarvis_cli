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
| **Executor** | Executa o plano (edits, shell, etc.) e cria PR |

## Comandos CLI

### Exec (jarvis-exec)

```bash
# Resolver issue específica
jarvis resolve owner/repo --issue 42

# Escanear e resolver issues com labels (ex: jarvis-auto)
jarvis resolve owner/repo
```

**Requisitos**:
- `GITHUB_TOKEN` ou `GITHUB_PAT` definido no ambiente
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
5. **Output**: PR criado, issue comentada, eventos `EnteredIssueResolverMode` / `IssueResolverOutputEvent`

## Eventos de Rollout

- `EnteredIssueResolverMode(IssueResolverRequest)` — persistido em rollout
- `ExitedReviewMode` — quando sai do modo

## Referências

- [GitHub Integration](./github-integration.md)
- [CLI Autonomous Commands](./cli-autonomous-commands.md)
- [AUTONOMY_IMPLEMENTATION_STATUS.md](../AUTONOMY_IMPLEMENTATION_STATUS.md)

---

**Última atualização**: 2026-03-10
