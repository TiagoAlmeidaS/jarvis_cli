# Composer Orchestration

## Visão Geral

O sistema de Composer Orchestration permite que o Jarvis CLI orquestre múltiplas ações complexas de forma coordenada. O sistema analisa dependências entre ações, executa ações em paralelo quando possível, e gerencia rollback em caso de falhas.

O sistema inclui:
- **Orquestração**: Coordena múltiplas ações
- **Análise de Dependências**: Identifica dependências entre ações
- **Execução Paralela**: Executa ações independentes em paralelo
- **Rollback**: Reverte ações em caso de falha
- **Transações**: Agrupa ações em transações atômicas

## Motivação

Problemas que o sistema resolve:

1. **Complexidade**: Gerencia tarefas complexas com múltiplas ações
2. **Eficiência**: Executa ações em paralelo quando possível
3. **Confiabilidade**: Garante consistência através de rollback
4. **Coordenação**: Coordena ações que dependem umas das outras
5. **Atomicidade**: Garante que ações relacionadas são executadas juntas

## Arquitetura

### Componentes Principais

```
┌─────────────────────────────────────────────────────────────┐
│              Composer Orchestration                         │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐ │
│  │ Composer     │───▶│ Dependency   │───▶│ Action       │ │
│  │ Mode         │    │ Graph        │    │ Executor     │ │
│  └──────────────┘    └──────────────┘    └──────────────┘ │
│         │                    │                    │         │
│         ▼                    ▼                    ▼         │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐ │
│  │ Action       │    │ Parallel     │    │ Rollback     │ │
│  │ Scheduler    │    │ Executor     │    │ Manager      │ │
│  └──────────────┘    └──────────────┘    └──────────────┘ │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### Fluxo de Dados

1. **Análise**:
   - Ações são analisadas
   - Dependências são identificadas
   - Grafo de dependências é construído

2. **Agendamento**:
   - Ações são ordenadas topologicamente
   - Ações independentes são identificadas
   - Plano de execução é criado

3. **Execução**:
   - Ações são executadas conforme plano
   - Ações independentes executam em paralelo
   - Progresso é rastreado

4. **Rollback**:
   - Se falha ocorre, ações são revertidas
   - Estado anterior é restaurado
   - Erro é reportado

### Integrações

- **Planning Engine**: Usa planos como entrada
- **Dependency Analyzer**: Para análise de dependências
- **Action Executors**: Para execução de ações específicas
- **Undo/Redo**: Para rollback

## Especificação Técnica

### APIs e Interfaces

```rust
// Composer trait
pub trait Composer: Send + Sync {
    async fn compose(
        &self,
        actions: Vec<Action>,
        options: ComposeOptions,
    ) -> Result<CompositionResult>;
    
    async fn execute(
        &self,
        composition: &Composition,
    ) -> Result<ExecutionResult>;
}

// Dependency graph builder
pub trait DependencyGraphBuilder: Send + Sync {
    fn build_graph(
        &self,
        actions: &[Action],
    ) -> Result<DependencyGraph>;
    
    fn find_independent_actions(
        &self,
        graph: &DependencyGraph,
    ) -> Vec<Vec<ActionId>>;
}
```

### Estruturas de Dados

```rust
pub struct Composition {
    pub id: CompositionId,
    pub actions: Vec<Action>,
    pub dependency_graph: DependencyGraph,
    pub execution_plan: ExecutionPlan,
    pub created_at: DateTime<Utc>,
}

pub type CompositionId = Uuid;
pub type ActionId = Uuid;

pub struct Action {
    pub id: ActionId,
    pub name: String,
    pub action_type: ActionType,
    pub dependencies: Vec<ActionId>,
    pub parameters: HashMap<String, Value>,
    pub rollback_action: Option<RollbackAction>,
}

pub enum ActionType {
    FileOperation(FileOperation),
    Command(CommandAction),
    Refactoring(RefactoringAction),
    Test(TestAction),
    Custom(String),
}

pub struct DependencyGraph {
    pub nodes: Vec<Action>,
    pub edges: Vec<(ActionId, ActionId)>,  // (from, to)
}

pub struct ExecutionPlan {
    pub phases: Vec<ExecutionPhase>,
    pub estimated_duration: Duration,
}

pub struct ExecutionPhase {
    pub phase_number: usize,
    pub actions: Vec<ActionId>,
    pub parallel: bool,
}

pub struct RollbackAction {
    pub action_type: ActionType,
    pub parameters: HashMap<String, Value>,
}

pub struct CompositionResult {
    pub composition: Composition,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

pub struct ExecutionResult {
    pub composition_id: CompositionId,
    pub success: bool,
    pub actions_completed: usize,
    pub actions_failed: usize,
    pub actions_rolled_back: usize,
    pub duration: Duration,
    pub results: Vec<ActionResult>,
}

pub struct ActionResult {
    pub action_id: ActionId,
    pub success: bool,
    pub output: Option<String>,
    pub error: Option<String>,
    pub duration: Duration,
    pub rolled_back: bool,
}
```

### Algoritmos

#### Construção de Grafo de Dependências

1. Criar nó para cada ação
2. Identificar dependências explícitas
3. Identificar dependências implícitas (arquivos, símbolos)
4. Adicionar arestas ao grafo
5. Detectar ciclos

#### Agendamento de Execução

1. Ordenar ações topologicamente
2. Agrupar ações independentes em fases
3. Estimar duração de cada fase
4. Criar plano de execução

#### Execução Paralela

1. Identificar ações prontas para execução (sem dependências pendentes)
2. Executar ações prontas em paralelo
3. Aguardar conclusão antes de próxima fase
4. Tratar falhas e executar rollback

## Comandos CLI

### `jarvis compose <actions-file>`

Cria uma composição de ações.

**Exemplo:**
```bash
jarvis compose actions.json
```

### `jarvis compose execute <composition-file>`

Executa uma composição.

**Opções:**
- `--dry-run`: Mostrar plano sem executar
- `--parallel`: Executar ações independentes em paralelo
- `--no-rollback`: Desabilitar rollback em caso de falha

**Exemplo:**
```bash
jarvis compose execute composition.json
jarvis compose execute composition.json --dry-run
jarvis compose execute composition.json --parallel
```

### `jarvis compose analyze <actions-file>`

Analisa dependências de ações.

**Exemplo:**
```bash
jarvis compose analyze actions.json
```

## Exemplos de Uso

### Exemplo 1: Composição Simples

```json
// actions.json
{
  "actions": [
    {
      "id": "action-1",
      "name": "Create file",
      "type": "file-operation",
      "operation": "create",
      "path": "src/utils.rs"
    },
    {
      "id": "action-2",
      "name": "Modify file",
      "type": "file-operation",
      "operation": "modify",
      "path": "src/main.rs",
      "depends_on": ["action-1"]
    }
  ]
}
```

```bash
$ jarvis compose execute actions.json

Analisando dependências...
✓ 2 ações identificadas
✓ 1 dependência encontrada

Executando:
✓ Fase 1: action-1 (Create file)
✓ Fase 2: action-2 (Modify file)

✓ Composição concluída com sucesso
```

### Exemplo 2: Execução Paralela

```bash
$ jarvis compose execute actions.json --parallel

Analisando dependências...
✓ 4 ações identificadas
✓ 2 ações podem executar em paralelo

Executando:
✓ Fase 1: action-1, action-2 (paralelo)
✓ Fase 2: action-3
✓ Fase 3: action-4

✓ Composição concluída em 2.3s (vs 4.1s sequencial)
```

## Considerações de Implementação

### Dependências

**Crates Rust necessários:**
- `uuid` - IDs únicos
- `serde` / `serde_json` - Serialização
- `tokio` - Async runtime e execução paralela
- `petgraph` - Para grafo de dependências
- `anyhow` / `thiserror` - Error handling
- `chrono` - Timestamps

**Integrações:**
- Planning Engine para planos
- Dependency Analyzer para análise
- Action Executors específicos
- Undo/Redo para rollback

### Desafios Técnicos

1. **Detecção de Dependências**: Como identificar dependências automaticamente?
   - **Solução**: Análise estática de código
   - Heurísticas baseadas em arquivos/símbolos
   - Permitir dependências explícitas

2. **Execução Paralela**: Como garantir segurança em execução paralela?
   - **Solução**: Isolar ações em sandboxes
   - Validar que ações são realmente independentes
   - Limitar recursos por ação

3. **Rollback**: Como reverter ações complexas?
   - **Solução**: Cada ação define rollback
   - Executar rollbacks em ordem reversa
   - Integrar com sistema undo/redo

4. **Atomicidade**: Como garantir atomicidade de transações?
   - **Solução**: Checkpoints antes de cada fase
   - Rollback completo em caso de falha
   - Validação antes de commit

### Performance

- **Parallel Execution**: Executar ações independentes em paralelo
- **Caching**: Cachear resultados de análises
- **Incremental**: Re-executar apenas ações afetadas

### Segurança

- **Sandboxing**: Executar ações em ambiente isolado
- **Validation**: Validar ações antes de executar
- **Rollback**: Sempre permitir rollback
- **Limits**: Limitar recursos por ação

## Roadmap de Implementação

### Fase 1: Core Infrastructure (Sprint 1)

- [ ] Definir estruturas de dados (`Composition`, `Action`, `DependencyGraph`)
- [ ] Implementar `DependencyGraphBuilder`
- [ ] Implementar ordenação topológica
- [ ] Comando `jarvis compose analyze`

### Fase 2: Execution (Sprint 2)

- [ ] Implementar `Composer`
- [ ] Execução sequencial básica
- [ ] Sistema de rollback básico
- [ ] Comando `jarvis compose execute`

### Fase 3: Parallel Execution (Sprint 3)

- [ ] Identificação de ações independentes
- [ ] Execução paralela
- [ ] Gerenciamento de recursos
- [ ] Otimizações de performance

### Fase 4: Advanced Features (Sprint 4)

- [ ] Transações atômicas
- [ ] Rollback avançado
- [ ] Analytics e métricas
- [ ] Integração com planning engine

## Referências

### Código Base (.NET)

- `Jarvis.CLI/Agentic/Composer/ComposerMode.cs` - Modo composer
- `Jarvis.CLI/Agentic/Composer/DependencyGraph.cs` - Grafo de dependências
- `Jarvis.CLI/Commands/ComposerCommand.cs` - Comando composer

### Documentação Externa

- [Topological Sorting](https://en.wikipedia.org/wiki/Topological_sorting)
- [Petgraph (Rust)](https://docs.rs/petgraph/latest/petgraph/)
- [Tokio Parallel Execution](https://tokio.rs/tokio/tutorial/spawning)
- [Jarvis CLI Planning Engine](./planning-engine.md) - Sistema relacionado

---

**Status**: 📝 Planejado  
**Prioridade**: 🟢 Baixa  
**Última atualização**: 2026-01-20
