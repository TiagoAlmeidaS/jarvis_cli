# Planning Engine

## Visão Geral

O Planning Engine permite que o Jarvis CLI crie planos estruturados e detalhados para tarefas complexas. O sistema analisa requisições do usuário, identifica dependências, e cria planos de execução passo-a-passo que podem ser revisados e executados.

O sistema inclui:
- **Análise de Requisições**: Entende o escopo e complexidade da tarefa
- **Geração de Planos**: Cria planos estruturados com passos e dependências
- **Análise de Dependências**: Identifica dependências entre passos
- **Execução de Planos**: Executa planos passo-a-passo
- **Validação**: Valida planos antes e durante execução

## Motivação

Problemas que o sistema resolve:

1. **Complexidade**: Quebra tarefas complexas em passos gerenciáveis
2. **Clareza**: Torna o processo de implementação claro e transparente
3. **Validação**: Permite revisar planos antes de executar
4. **Dependências**: Identifica e respeita dependências entre passos
5. **Rastreabilidade**: Rastreia progresso através do plano

## Arquitetura

### Componentes Principais

```
┌─────────────────────────────────────────────────────────────┐
│                  Planning Engine                           │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐ │
│  │ Planning     │───▶│ Dependency   │───▶│ Plan         │ │
│  │ Engine       │    │ Analyzer     │    │ Executor     │ │
│  └──────────────┘    └──────────────┘    └──────────────┘ │
│         │                    │                    │         │
│         ▼                    ▼                    ▼         │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐ │
│  │ Context      │    │ Step         │    │ Validation   │ │
│  │ Gatherer     │    │ Generator    │    │ Engine       │ │
│  └──────────────┘    └──────────────┘    └──────────────┘ │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### Fluxo de Dados

1. **Análise**:
   - Requisição do usuário é analisada
   - Contexto do projeto é coletado
   - Complexidade é estimada

2. **Geração de Plano**:
   - Passos são identificados
   - Dependências são analisadas
   - Ordem de execução é determinada

3. **Revisão**:
   - Plano é apresentado ao usuário
   - Usuário pode aprovar ou modificar
   - Validações são executadas

4. **Execução**:
   - Passos são executados sequencialmente
   - Dependências são respeitadas
   - Progresso é rastreado

### Integrações

- **LLM Gateway**: Usa LLM para análise e geração de planos
- **Context Engine**: Coleta contexto do projeto
- **File Operations**: Identifica arquivos a modificar
- **Test Runner**: Valida mudanças após cada passo

## Especificação Técnica

### APIs e Interfaces

```rust
// Planning engine trait
pub trait PlanningEngine: Send + Sync {
    async fn create_plan(
        &self,
        user_request: &str,
        context: &PlanningContext,
    ) -> Result<ExecutionPlan>;
    
    async fn review_plan(
        &self,
        plan: &ExecutionPlan,
    ) -> Result<PlanReview>;
    
    async fn execute_plan(
        &self,
        plan: &ExecutionPlan,
        options: ExecutionOptions,
    ) -> Result<ExecutionResult>;
}

// Dependency analyzer trait
pub trait DependencyAnalyzer: Send + Sync {
    fn analyze_dependencies(
        &self,
        steps: &[PlanStep],
    ) -> Result<DependencyGraph>;
    
    fn order_steps(
        &self,
        graph: &DependencyGraph,
    ) -> Vec<PlanStep>;
}
```

### Estruturas de Dados

```rust
pub struct ExecutionPlan {
    pub id: PlanId,
    pub title: String,
    pub description: String,
    pub steps: Vec<PlanStep>,
    pub files_to_modify: Vec<PathBuf>,
    pub estimated_impact: ImpactEstimate,
    pub risks: Vec<Risk>,
    pub created_at: DateTime<Utc>,
}

pub type PlanId = Uuid;

pub struct PlanStep {
    pub id: StepId,
    pub order: usize,
    pub title: String,
    pub description: String,
    pub action: StepAction,
    pub dependencies: Vec<StepId>,
    pub acceptance_criteria: Vec<String>,
    pub estimated_complexity: Complexity,
    pub files_affected: Vec<PathBuf>,
}

pub enum StepAction {
    CreateFile { path: PathBuf, content: String },
    ModifyFile { path: PathBuf, changes: Vec<FileChange> },
    DeleteFile { path: PathBuf },
    RunCommand { command: String, args: Vec<String> },
    RunTests { files: Vec<PathBuf> },
    Custom { name: String, params: HashMap<String, Value> },
}

pub enum Complexity {
    Low,
    Medium,
    High,
}

pub struct ImpactEstimate {
    pub files_affected: usize,
    pub lines_added: usize,
    pub lines_modified: usize,
    pub lines_deleted: usize,
    pub risk_level: RiskLevel,
}

pub enum RiskLevel {
    Low,
    Medium,
    High,
}

pub struct Risk {
    pub description: String,
    pub severity: RiskLevel,
    pub mitigation: Option<String>,
}

pub struct DependencyGraph {
    pub nodes: Vec<PlanStep>,
    pub edges: Vec<(StepId, StepId)>,  // (from, to)
}

pub struct ExecutionResult {
    pub plan_id: PlanId,
    pub steps_completed: usize,
    pub steps_failed: usize,
    pub steps_skipped: usize,
    pub total_steps: usize,
    pub duration: Duration,
    pub results: Vec<StepResult>,
}

pub struct StepResult {
    pub step_id: StepId,
    pub success: bool,
    pub output: Option<String>,
    pub error: Option<String>,
    pub duration: Duration,
}
```

### Algoritmos

#### Análise de Dependências

1. Identificar dependências explícitas (declaradas)
2. Identificar dependências implícitas (arquivos, funções)
3. Construir grafo de dependências
4. Detectar ciclos
5. Ordenar topologicamente

#### Geração de Plano

1. Analisar requisição usando LLM
2. Identificar arquivos a modificar
3. Gerar passos usando LLM
4. Analisar dependências
5. Estimar impacto e riscos

## Comandos CLI

### `jarvis plan <request>`

Cria um plano para uma requisição.

**Opções:**
- `--output <file>`: Salvar plano em arquivo
- `--format <format>`: Formato de saída (json, yaml, markdown)

**Exemplo:**
```bash
jarvis plan "refactor authentication to use JWT"
jarvis plan "add user management feature" --output plan.json
```

### `jarvis plan execute <plan-file>`

Executa um plano salvo.

**Opções:**
- `--dry-run`: Mostrar o que seria executado sem aplicar
- `--step <step-id>`: Executar apenas um passo específico
- `--skip-validation`: Pular validações

**Exemplo:**
```bash
jarvis plan execute plan.json
jarvis plan execute plan.json --dry-run
jarvis plan execute plan.json --step "step-1"
```

### `jarvis plan review <plan-file>`

Revisa um plano antes de executar.

**Exemplo:**
```bash
jarvis plan review plan.json
```

## Exemplos de Uso

### Exemplo 1: Criar Plano

```bash
$ jarvis plan "refactor authentication to use JWT"

Plano criado:
1. Analisar código atual de autenticação
2. Criar módulo JWT
3. Migrar funções de autenticação
4. Atualizar testes
5. Validar mudanças

Arquivos afetados: 5
Risco estimado: Médio
```

### Exemplo 2: Executar Plano

```bash
$ jarvis plan execute plan.json

Executando plano...
✓ Passo 1/5: Analisar código atual
✓ Passo 2/5: Criar módulo JWT
✓ Passo 3/5: Migrar funções
⏸ Passo 4/5: Aguardando aprovação...
```

## Considerações de Implementação

### Dependências

**Crates Rust necessários:**
- `uuid` - IDs únicos
- `serde` / `serde_json` - Serialização
- `tokio` - Async runtime
- `petgraph` - Para grafo de dependências
- `anyhow` / `thiserror` - Error handling
- `chrono` - Timestamps

**Integrações:**
- LLM Gateway para análise e geração
- Context Engine para coleta de contexto
- File Operations para modificações
- Test Runner para validação

### Desafios Técnicos

1. **Análise de Requisições**: Como entender requisições complexas?
   - **Solução**: Usar LLM com contexto do projeto
   - Prompt engineering para análise estruturada

2. **Dependências**: Como identificar dependências automaticamente?
   - **Solução**: Análise estática de código
   - Heurísticas baseadas em imports/references
   - LLM para análise semântica

3. **Validação**: Como validar planos antes de executar?
   - **Solução**: Verificar arquivos existem
   - Validar estrutura de passos
   - Detectar conflitos potenciais

4. **Rollback**: Como reverter se algo der errado?
   - **Solução**: Integrar com sistema undo/redo
   - Criar checkpoint antes de cada passo
   - Permitir rollback parcial

### Performance

- **Caching**: Cachear análises de contexto
- **Parallel Execution**: Executar passos independentes em paralelo
- **Incremental**: Atualizar planos incrementalmente

### Segurança

- **Validação**: Validar ações antes de executar
- **Sandboxing**: Executar em ambiente isolado
- **Approval**: Requerer aprovação para ações destrutivas

## Roadmap de Implementação

### Fase 1: Core Infrastructure (Sprint 1)

- [ ] Definir estruturas de dados (`ExecutionPlan`, `PlanStep`)
- [ ] Implementar `PlanningEngine` básico
- [ ] Integração com LLM para análise
- [ ] Comando `jarvis plan`

### Fase 2: Dependency Analysis (Sprint 2)

- [ ] Implementar `DependencyAnalyzer`
- [ ] Análise de dependências de arquivos
- [ ] Ordenação topológica
- [ ] Detecção de ciclos

### Fase 3: Execution (Sprint 3)

- [ ] Implementar `PlanExecutor`
- [ ] Execução passo-a-passo
- [ ] Validação entre passos
- [ ] Comando `jarvis plan execute`

### Fase 4: Advanced Features (Sprint 4)

- [ ] Revisão interativa de planos
- [ ] Rollback automático
- [ ] Execução paralela
- [ ] Analytics e métricas

## Referências

### Código Base (.NET)

- `Jarvis.CLI/Agentic/Planning/PlanningEngine.cs` - Engine principal
- `Jarvis.CLI/Agentic/Planning/PlanExecutor.cs` - Executor de planos
- `Jarvis.CLI/Agentic/Planning/DependencyAnalyzer.cs` - Análise de dependências

### Documentação Externa

- [Topological Sorting](https://en.wikipedia.org/wiki/Topological_sorting)
- [Petgraph (Rust)](https://docs.rs/petgraph/latest/petgraph/)
- [Jarvis CLI Undo/Redo](./undo-redo-system.md) - Sistema de rollback

---

**Status**: 📝 Planejado  
**Prioridade**: 🟢 Baixa  
**Última atualização**: 2026-01-20
