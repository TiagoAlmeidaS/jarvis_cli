# Sistema de Agents Registry

## Visão Geral

O sistema de Agents Registry permite gerenciar e usar diferentes "agents" (agentes) especializados para diferentes tarefas. Cada agent tem personalidade, instruções e capacidades específicas, permitindo que o Jarvis CLI escolha o agent mais apropriado para cada situação.

O sistema inclui:
- **Registry de Agents**: Registro centralizado de agents disponíveis
- **Agent Matching**: Seleção automática do agent mais adequado
- **Métricas de Agents**: Tracking de uso e performance
- **Agents Customizados**: Capacidade de criar agents personalizados

## Motivação

Problemas que o sistema resolve:

1. **Especialização**: Diferentes tarefas requerem diferentes abordagens
2. **Qualidade**: Agents especializados produzem melhores resultados
3. **Eficiência**: Escolher o agent certo evita tentativas desnecessárias
4. **Flexibilidade**: Permite criar agents para casos de uso específicos
5. **Aprendizado**: Métricas ajudam a melhorar seleção de agents

## Arquitetura

### Componentes Principais

```
┌─────────────────────────────────────────────────────────────┐
│                  Agents Registry System                     │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐ │
│  │ Agent        │───▶│ Agent        │───▶│ Agent        │ │
│  │ Registry     │    │ Matcher      │    │ Executor     │ │
│  └──────────────┘    └──────────────┘    └──────────────┘ │
│         │                    │                    │         │
│         ▼                    ▼                    ▼         │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐ │
│  │ Agent        │    │ Agent        │    │ Agent        │ │
│  │ Templates    │    │ Metrics      │    │ Loader       │ │
│  └──────────────┘    └──────────────┘    └──────────────┘ │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### Fluxo de Dados

1. **Registro de Agents**:
   - Agents são registrados no registry
   - Cada agent tem metadata (nome, descrição, keywords)
   - Agents podem ser carregados de templates ou definidos dinamicamente

2. **Matching de Agents**:
   - Query do usuário é analisada
   - Keywords e contexto são extraídos
   - Agents são pontuados por relevância
   - Agent com maior score é selecionado

3. **Execução**:
   - Agent selecionado é carregado
   - Instruções do agent são aplicadas ao prompt
   - LLM é chamado com contexto do agent
   - Resultado é retornado

4. **Métricas**:
   - Uso de cada agent é rastreado
   - Taxa de sucesso é calculada
   - Performance é medida

### Integrações

- **LLM Gateway**: Agents modificam prompts enviados ao LLM
- **Template System**: Agents podem ser definidos via templates Markdown
- **Metrics System**: Integração com sistema de métricas existente
- **Config System**: Configuração de agents via config.toml

## Especificação Técnica

### APIs e Interfaces

```rust
// Agent registry trait
pub trait AgentRegistry: Send + Sync {
    fn register_agent(&mut self, agent: Agent) -> Result<()>;
    fn get_agent(&self, name: &str) -> Option<&Agent>;
    fn get_all_agents(&self) -> Vec<&Agent>;
    fn search_agents(&self, query: &str) -> Vec<&Agent>;
    fn match_agent(&self, context: &AgentContext) -> Option<&Agent>;
}

// Agent matcher trait
pub trait AgentMatcher: Send + Sync {
    fn match_agent(
        &self,
        query: &str,
        context: &AgentContext,
        agents: &[&Agent],
    ) -> Option<MatchedAgent>;
    
    fn score_agent(
        &self,
        agent: &Agent,
        query: &str,
        context: &AgentContext,
    ) -> f32;
}

// Agent executor trait
pub trait AgentExecutor: Send + Sync {
    async fn execute_with_agent(
        &self,
        agent: &Agent,
        user_query: &str,
        context: &AgentContext,
    ) -> Result<AgentResponse>;
}
```

### Estruturas de Dados

```rust
pub struct Agent {
    pub id: AgentId,
    pub name: String,
    pub description: String,
    pub keywords: Vec<String>,
    pub instructions: String,
    pub model_preference: Option<String>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<usize>,
    pub tools: Vec<String>,  // Tools permitidos para este agent
    pub metadata: AgentMetadata,
}

pub type AgentId = String;

pub struct AgentMetadata {
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub author: Option<String>,
    pub version: String,
    pub category: AgentCategory,
}

pub enum AgentCategory {
    Planner,
    Developer,
    Reviewer,
    Tester,
    Documenter,
    Custom(String),
}

pub struct AgentContext {
    pub user_query: String,
    pub conversation_history: Vec<Message>,
    pub file_context: Vec<FileContext>,
    pub project_type: Option<String>,
    pub language: Option<String>,
}

pub struct MatchedAgent {
    pub agent: Agent,
    pub score: f32,
    pub reasons: Vec<String>,
}

pub struct AgentResponse {
    pub content: String,
    pub agent_used: AgentId,
    pub tokens_used: TokenUsage,
    pub tools_called: Vec<ToolCall>,
}

pub struct AgentMetrics {
    pub agent_id: AgentId,
    pub usage_count: u64,
    pub success_count: u64,
    pub failure_count: u64,
    pub average_tokens: f64,
    pub average_response_time_ms: f64,
    pub last_used: Option<DateTime<Utc>>,
}
```

### Algoritmos

#### Agent Matching

1. **Extração de Keywords**: Extrair keywords da query do usuário
2. **Scoring**: Para cada agent:
   - Calcular match de keywords (TF-IDF ou similar)
   - Considerar categoria do agent
   - Considerar histórico de uso
   - Considerar taxa de sucesso
3. **Seleção**: Escolher agent com maior score
4. **Fallback**: Se nenhum agent tem score suficiente, usar agent padrão

#### Scoring Formula

```
score = (
    keyword_match_score * 0.4 +
    category_relevance * 0.3 +
    success_rate * 0.2 +
    recency_bonus * 0.1
)
```

## Comandos CLI

### `jarvis agents list`

Lista todos os agents registrados.

**Opções:**
- `--format <format>`: Formato de saída (table, json)
- `--category <category>`: Filtrar por categoria

**Exemplo:**
```bash
jarvis agents list
jarvis agents list --category planner
jarvis agents list --format json
```

### `jarvis agents match <query>`

Encontra o agent mais adequado para uma query.

**Opções:**
- `--show-scores`: Mostrar scores de todos os agents
- `--context <path>`: Incluir contexto de arquivo/diretório

**Exemplo:**
```bash
jarvis agents match "refactor this code"
jarvis agents match "plan a new feature" --show-scores
```

### `jarvis agents metrics`

Mostra métricas de uso dos agents.

**Opções:**
- `--agent <name>`: Métricas de um agent específico
- `--format <format>`: Formato de saída

**Exemplo:**
```bash
jarvis agents metrics
jarvis agents metrics --agent planner
```

### `jarvis agents show <name>`

Mostra detalhes de um agent específico.

**Exemplo:**
```bash
jarvis agents show planner
jarvis agents show developer
```

## Exemplos de Uso

### Exemplo 1: Uso Automático

Durante conversas, o sistema automaticamente seleciona o agent:

```
User: "Planeje uma nova feature de autenticação"

Jarvis: [Seleciona agent "Planner"]
        [Usa instruções do Planner]
        "Vou criar um plano detalhado..."
```

### Exemplo 2: Listar Agents

```bash
$ jarvis agents list

Agents disponíveis:
1. planner - Cria planos detalhados de implementação
2. developer - Implementa código seguindo boas práticas
3. reviewer - Revisa código e sugere melhorias
4. tester - Cria e executa testes
```

### Exemplo 3: Ver Métricas

```bash
$ jarvis agents metrics

Agent Metrics:
┌─────────────┬─────────────┬──────────────┬─────────────┐
│ Agent       │ Usage       │ Success Rate │ Avg Tokens  │
├─────────────┼─────────────┼──────────────┼─────────────┤
│ planner     │ 45          │ 92%          │ 2,340       │
│ developer   │ 120         │ 88%          │ 3,120       │
│ reviewer    │ 30          │ 95%          │ 1,890       │
└─────────────┴─────────────┴──────────────┴─────────────┘
```

## Considerações de Implementação

### Dependências

**Crates Rust necessários:**
- `serde` / `serde_json` - Serialização
- `tokio` - Async runtime
- `anyhow` / `thiserror` - Error handling
- `chrono` - Timestamps
- `uuid` - IDs únicos (opcional)

**Crates para matching:**
- `tantivy` ou `meilisearch` - Para busca full-text (opcional)
- Ou implementar matching simples com scoring manual

### Desafios Técnicos

1. **Matching Inteligente**: Como escolher o agent certo?
   - **Solução**: Usar scoring baseado em keywords, categoria e histórico
   - Considerar contexto do projeto
   - Aprender com feedback do usuário

2. **Templates de Agents**: Como definir agents?
   - **Solução**: Usar arquivos Markdown (como já existe)
   - Suportar carregamento de `~/.jarvis/agents/`
   - Permitir agents inline em config.toml

3. **Métricas**: Como rastrear uso e performance?
   - **Solução**: Integrar com sistema de métricas existente
   - Persistir em SQLite (state runtime)
   - Calcular métricas agregadas

4. **Fallback**: O que fazer se nenhum agent match?
   - **Solução**: Usar agent padrão (general-purpose)
   - Permitir que usuário escolha manualmente
   - Aprender com escolhas manuais

### Performance

- **Lazy Loading**: Carregar agents apenas quando necessário
- **Caching**: Cachear agents frequentes
- **Indexação**: Indexar keywords para busca rápida

### Segurança

- **Validação**: Validar instruções de agents customizados
- **Sandboxing**: Executar agents em ambiente seguro
- **Limites**: Limitar recursos usados por agent

## Roadmap de Implementação

### Fase 1: Core Infrastructure (Sprint 1)

- [ ] Definir estruturas de dados (`Agent`, `AgentRegistry`)
- [ ] Implementar `AgentRegistry` básico
- [ ] Carregar agents de templates existentes
- [ ] Comando `agents list`

### Fase 2: Matching System (Sprint 2)

- [ ] Implementar `AgentMatcher`
- [ ] Algoritmo de scoring
- [ ] Comando `agents match`
- [ ] Integração com LLM Gateway

### Fase 3: Metrics (Sprint 3)

- [ ] Implementar tracking de métricas
- [ ] Persistência de métricas
- [ ] Comando `agents metrics`
- [ ] Dashboard de métricas

### Fase 4: Custom Agents (Sprint 4)

- [ ] Suporte a agents customizados
- [ ] Validação de agents
- [ ] Comando para criar agents
- [ ] Documentação de como criar agents

### Fase 5: Advanced Features (Sprint 5)

- [ ] Learning de preferências do usuário
- [ ] Agents compostos
- [ ] A/B testing de agents
- [ ] Analytics avançados

## Referências

### Código Base (.NET)

- `Jarvis.CLI/Commands/AgentsCommand.cs` - Comandos de agents
- `Jarvis.CLI/Services/IAgentRegistry` - Interface do registry

### Código Existente (Rust)

- `jarvis-rs/core/templates/agents/planner.md` - Template do Planner
- `jarvis-rs/core/templates/agents/developer.md` - Template do Developer
- `jarvis-rs/core/templates/agents/reviewer.md` - Template do Reviewer

### Documentação Externa

- [Agent Patterns](https://www.patterns.dev/posts/agent-patterns/)
- [Tantivy (Rust Search)](https://github.com/quickwit-oss/tantivy)
- [Jarvis CLI Templates](../templates.md) - Sistema de templates existente

---

**Status**: 📝 Planejado  
**Prioridade**: 🟡 Média  
**Última atualização**: 2026-01-20
