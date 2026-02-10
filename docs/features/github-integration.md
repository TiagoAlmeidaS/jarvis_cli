# Integração GitHub

## Visão Geral

A integração GitHub permite que o Jarvis CLI interaja com repositórios GitHub através da API do GitHub. O sistema fornece tools que o LLM pode usar para criar, listar, atualizar e gerenciar issues, além de outras operações relacionadas a repositórios.

O sistema inclui:
- **Criação de Issues**: Criar issues no GitHub
- **Listagem de Issues**: Listar e filtrar issues
- **Atualização de Issues**: Atualizar issues existentes
- **Vínculo de Issues**: Vincular issues relacionadas
- **Operações de Repositório**: Listar repositórios, obter informações

## Motivação

Problemas que o sistema resolve:

1. **Automação de Issues**: Criar issues automaticamente baseado em análise de código
2. **Rastreamento**: Vincular mudanças de código a issues
3. **Integração**: Integrar workflow de desenvolvimento com GitHub
4. **Automação**: Automatizar tarefas repetitivas relacionadas a GitHub
5. **Visibilidade**: Melhorar visibilidade de problemas e melhorias

## Arquitetura

### Componentes Principais

```
┌─────────────────────────────────────────────────────────────┐
│                  GitHub Integration                         │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐ │
│  │ GitHub       │───▶│ GitHub       │───▶│ Tool         │ │
│  │ API Client   │    │ Tools        │    │ Registry     │ │
│  └──────────────┘    └──────────────┘    └──────────────┘ │
│         │                    │                    │         │
│         ▼                    ▼                    ▼         │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐ │
│  │ Authentication│    │ Error        │    │ Validation   │ │
│  │ Manager      │    │ Handler      │    │ Helper       │ │
│  └──────────────┘    └──────────────┘    └──────────────┘ │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### Fluxo de Dados

1. **Autenticação**:
   - Obter token GitHub (via secrets manager)
   - Validar token
   - Criar cliente GitHub autenticado

2. **Execução de Tool**:
   - LLM chama tool GitHub (ex: `github_create_issue`)
   - Validar parâmetros
   - Fazer requisição à API GitHub
   - Processar resposta
   - Retornar resultado ao LLM

3. **Tratamento de Erros**:
   - Capturar erros da API
   - Traduzir para mensagens amigáveis
   - Retornar erro estruturado

### Integrações

- **GitHub API**: Comunicação com GitHub via REST API
- **Secrets Manager**: Armazenamento seguro de tokens
- **Tool Registry**: Registro de tools para LLM
- **Error Handling**: Tratamento centralizado de erros

## Especificação Técnica

### APIs e Interfaces

```rust
// GitHub tool handler trait
#[async_trait]
pub trait GitHubToolHandler: Send + Sync {
    async fn create_issue(
        &self,
        owner: &str,
        repo: &str,
        title: &str,
        body: Option<&str>,
        labels: Option<&[String]>,
        assignees: Option<&[String]>,
    ) -> Result<GitHubIssue>;
    
    async fn get_issue(
        &self,
        owner: &str,
        repo: &str,
        issue_number: u64,
    ) -> Result<GitHubIssue>;
    
    async fn list_issues(
        &self,
        owner: &str,
        repo: &str,
        state: Option<IssueState>,
        labels: Option<&[String]>,
    ) -> Result<Vec<GitHubIssue>>;
    
    async fn update_issue(
        &self,
        owner: &str,
        repo: &str,
        issue_number: u64,
        title: Option<&str>,
        body: Option<&str>,
        state: Option<IssueState>,
        labels: Option<&[String]>,
    ) -> Result<GitHubIssue>;
    
    async fn link_issues(
        &self,
        owner: &str,
        repo: &str,
        issue_number: u64,
        linked_issues: &[u64],
    ) -> Result<()>;
}
```

### Estruturas de Dados

```rust
pub struct GitHubIssue {
    pub number: u64,
    pub title: String,
    pub body: String,
    pub state: IssueState,
    pub labels: Vec<Label>,
    pub assignees: Vec<User>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub url: String,
}

pub enum IssueState {
    Open,
    Closed,
    All,
}

pub struct Label {
    pub name: String,
    pub color: String,
    pub description: Option<String>,
}

pub struct User {
    pub login: String,
    pub name: Option<String>,
    pub avatar_url: Option<String>,
}

pub struct GitHubError {
    pub message: String,
    pub status_code: Option<u16>,
    pub error_type: GitHubErrorType,
}

pub enum GitHubErrorType {
    Authentication,
    NotFound,
    Validation,
    RateLimit,
    ServerError,
    Unknown,
}
```

### Algoritmos

#### Validação de Parâmetros

1. Validar owner/repo (formato correto)
2. Validar título (não vazio, tamanho máximo)
3. Validar labels (existem no repositório?)
4. Validar assignees (são colaboradores?)
5. Retornar erros de validação claros

#### Tratamento de Erros

1. Capturar erro da API GitHub
2. Identificar tipo de erro (status code)
3. Traduzir para mensagem amigável
4. Incluir contexto útil (sugestões, links)
5. Retornar erro estruturado

## Comandos CLI

**Nota**: GitHub tools são principalmente usados pelo LLM durante conversas. Comandos CLI diretos podem ser adicionados para debugging/testes.

### `jarvis github test`

Testa conexão com GitHub API.

**Exemplo:**
```bash
jarvis github test
```

### `jarvis github auth`

Configura autenticação GitHub.

**Exemplo:**
```bash
jarvis github auth
```

## Exemplos de Uso

### Exemplo 1: Criar Issue Automaticamente

Durante uma conversa, o LLM pode criar issues:

```
User: "Encontrei um bug na função de autenticação"

Jarvis: [Analisa código]
        [Cria issue no GitHub]
        ✓ Issue #123 criada: "Bug na função de autenticação"
        https://github.com/user/repo/issues/123
```

### Exemplo 2: Vincular Issues

```
User: "Essa mudança resolve a issue #100"

Jarvis: [Vincula issue #100 à mudança atual]
        ✓ Issue #100 vinculada
```

### Exemplo 3: Listar Issues Abertas

```
User: "Quais issues estão abertas neste projeto?"

Jarvis: [Lista issues abertas]
        Issues abertas:
        1. #123 - Bug na função de autenticação
        2. #124 - Melhorar performance
        3. #125 - Adicionar testes
```

## Considerações de Implementação

### Dependências

**Crates Rust necessários:**
- `reqwest` - Cliente HTTP
- `serde` / `serde_json` - Serialização
- `tokio` - Async runtime
- `anyhow` / `thiserror` - Error handling
- `chrono` - Timestamps

**Crates opcionais:**
- `octocrab` - Cliente GitHub oficial (mais completo)
- Ou usar `reqwest` diretamente com GitHub API

### Desafios Técnicos

1. **Autenticação**: Como gerenciar tokens GitHub?
   - **Solução**: Usar `jarvis_secrets` existente
   - Armazenar token em `~/.jarvis/secrets/github_token`
   - Suportar GitHub App tokens também

2. **Rate Limiting**: GitHub tem limites de API
   - **Solução**: Implementar rate limiting
   - Cachear respostas quando apropriado
   - Retry com backoff exponencial

3. **Validação**: Validar parâmetros antes de chamar API
   - **Solução**: Validação local primeiro
   - Verificar labels/assignees existem
   - Validar formato de owner/repo

4. **Error Handling**: Traduzir erros da API
   - **Solução**: Mapear status codes para mensagens
   - Incluir contexto útil
   - Sugerir ações corretivas

### Performance

- **Caching**: Cachear listagens de issues (TTL curto)
- **Batch Operations**: Quando possível, fazer operações em batch
- **Async**: Todas operações devem ser assíncronas

### Segurança

- **Token Storage**: Armazenar tokens de forma segura
- **Validation**: Validar inputs antes de enviar à API
- **Scopes**: Solicitar apenas permissões necessárias
- **Rate Limiting**: Respeitar limites da API

## Status da Implementação Existente

### Implementação Atual (Rust)

O projeto já possui implementação básica em `jarvis-rs/core/src/tools/handlers/github.rs`:

**Funcionalidades existentes:**
- ✅ `create_issue` - Criar issues
- ✅ `comment_pr` - Comentar em PRs
- ✅ `list_repos` - Listar repositórios

**Funcionalidades faltantes (do .NET):**
- ❌ `get_issue` - Obter issue específica
- ❌ `list_issues` - Listar issues do repositório
- ❌ `update_issue` - Atualizar issue
- ❌ `link_issues` - Vincular issues

### Melhorias Recomendadas

1. **Adicionar tools faltantes**:
   - Implementar `get_issue`, `list_issues`, `update_issue`, `link_issues`

2. **Melhorar validação**:
   - Adicionar validação de parâmetros mais robusta
   - Verificar existência de labels/assignees

3. **Melhorar error handling**:
   - Mensagens de erro mais amigáveis
   - Sugestões de correção

4. **Adicionar comandos CLI**:
   - Comandos para testar e gerenciar autenticação
   - Comandos para debugging

## Roadmap de Implementação

### Fase 1: Completar Tools Básicas (Sprint 1)

- [ ] Implementar `get_issue` tool
- [ ] Implementar `list_issues` tool
- [ ] Implementar `update_issue` tool
- [ ] Implementar `link_issues` tool

### Fase 2: Melhorias (Sprint 2)

- [ ] Adicionar validação robusta de parâmetros
- [ ] Melhorar tratamento de erros
- [ ] Adicionar rate limiting
- [ ] Adicionar caching

### Fase 3: Comandos CLI (Sprint 3)

- [ ] Comando `jarvis github test`
- [ ] Comando `jarvis github auth`
- [ ] Comando `jarvis github list-issues`
- [ ] Comando `jarvis github create-issue`

### Fase 4: Features Avançadas (Sprint 4)

- [ ] Suporte a GitHub Apps
- [ ] Webhooks (opcional)
- [ ] Integração com PRs
- [ ] Analytics de issues

## Referências

### Código Base (.NET)

- `Jarvis.CLI/Tools/GitHubCreateIssueTool.cs` - Criar issues
- `Jarvis.CLI/Tools/GitHubGetIssueTool.cs` - Obter issue
- `Jarvis.CLI/Tools/GitHubListIssuesTool.cs` - Listar issues
- `Jarvis.CLI/Tools/GitHubUpdateIssueTool.cs` - Atualizar issue
- `Jarvis.CLI/Tools/GitHubLinkIssuesTool.cs` - Vincular issues
- `Jarvis.CLI/Tools/Helpers/GitHubErrorHandler.cs` - Tratamento de erros
- `Jarvis.CLI/Tools/Helpers/GitHubValidationHelper.cs` - Validação

### Código Existente (Rust)

- `jarvis-rs/core/src/tools/handlers/github.rs` - Implementação atual

### Documentação Externa

- [GitHub REST API](https://docs.github.com/en/rest)
- [GitHub API Authentication](https://docs.github.com/en/rest/authentication)
- [Octocrab (Rust GitHub Client)](https://docs.rs/octocrab/latest/octocrab/)
- [Jarvis Secrets](../secrets.md) - Sistema de secrets existente

---

**Status**: 🚧 Parcialmente Implementado  
**Prioridade**: 🟡 Média  
**Última atualização**: 2026-01-20
