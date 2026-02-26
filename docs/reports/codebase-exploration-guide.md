# Guia de Exploração do Codebase para Análises

**Data**: 2026-02-18  
**Objetivo**: Documentar como o agente Jarvis deve explorar o codebase antes de fazer análises ou responder perguntas sobre o projeto.

---

## Problema Identificado

Análises genéricas baseadas apenas em documentação (como `AGENTS.md`) resultam em:

1. **Afirmações incorretas**: Dizer que componentes estão faltando quando na verdade existem
2. **Omissões críticas**: Não mencionar componentes importantes que já estão implementados
3. **Análise superficial**: Inferências genéricas em vez de análise baseada em código real

---

## Estrutura do Projeto Jarvis

Antes de explorar, entenda a organização:

```
jarvis-rs/
├── core/              # Lógica de negócio principal
│   ├── src/
│   │   ├── agent_loop/      # Agent loop (Think→Execute→Observe)
│   │   ├── agent/           # Agentes (Plan, Explore, Session)
│   │   ├── autonomous/      # Decision engine, planner, context
│   │   ├── rag/             # RAG system completo
│   │   ├── knowledge/       # Knowledge base
│   │   ├── intent/          # Intent detection
│   │   ├── skills/          # Skills system
│   │   ├── context_manager/ # Context management
│   │   └── jarvis.rs        # Core Jarvis implementation
├── daemon/            # Daemon autônomo (produção 24/7)
│   ├── src/
│   │   ├── executor.rs      # Proposal executor
│   │   ├── decision_engine.rs # Decision engine local
│   │   ├── pipelines/       # Pipelines de produção
│   │   └── main.rs          # Entry point + goal system
├── tui/               # Terminal UI
│   └── src/
│       └── chatwidget.rs    # Integração agent_loop
├── cli/               # CLI commands
└── protocol/          # Protocol definitions
```

## Processo de Exploração Recomendado

### Fase 1: Busca Semântica Inicial

Antes de fazer qualquer afirmação, use `codebase_search` para encontrar implementações relevantes:

**Exemplos práticos para o Jarvis**:

```rust
// Buscar por agent loop
codebase_search(
    query: "How does the agent loop work? What is the autonomous agent implementation?",
    target_directories: []
)
// → Deve encontrar: core/src/agent_loop/mod.rs

// Buscar por planejamento
codebase_search(
    query: "How does planning and task decomposition work?",
    target_directories: []
)
// → Deve encontrar: core/src/agent/plan.rs, core/src/autonomous/planner.rs

// Buscar por memória/contexto
codebase_search(
    query: "How does memory and context management work?",
    target_directories: []
)
// → Deve encontrar: core/src/agent/session.rs, core/src/context_manager/

// Buscar por daemon
codebase_search(
    query: "How does the daemon work? What autonomous features does it have?",
    target_directories: []
)
// → Deve encontrar: daemon/src/

// Buscar por RAG
codebase_search(
    query: "How does RAG and knowledge base work? How is information indexed?",
    target_directories: []
)
// → Deve encontrar: core/src/rag/, core/src/knowledge/
```

**Regra**: Sempre busque por termos relacionados antes de afirmar que algo não existe. Use perguntas completas em inglês para melhor resultado.

### Fase 2: Validação com Busca Direta

Após encontrar pistas semânticas, valide com buscas diretas:

**Padrões de arquivo para o Jarvis**:

```rust
// Buscar por módulos específicos
glob_file_search(glob_pattern: "**/agent_loop*.rs")
// → Encontra: core/src/agent_loop/mod.rs, tui/src/chatwidget/agent_loop_runner.rs

glob_file_search(glob_pattern: "**/plan*.rs")
// → Encontra: core/src/agent/plan.rs, core/src/autonomous/planner.rs

glob_file_search(glob_pattern: "**/rag/**/*.rs")
// → Encontra: core/src/rag/mod.rs, core/src/rag/indexer.rs, etc.

glob_file_search(glob_pattern: "**/daemon/**/*.rs")
// → Encontra: daemon/src/main.rs, daemon/src/executor.rs, etc.

// Buscar por símbolos/chaves
grep(pattern: "AgentLoop|agent_loop", path: "jarvis-rs")
// → Encontra usos em: tui/src/chatwidget.rs, core/src/config/types.rs

grep(pattern: "ExecutionPlan|PlanningEngine|PlanAgent", path: "jarvis-rs")
// → Encontra: core/src/autonomous/planner.rs, core/src/agent/plan.rs

grep(pattern: "KnowledgeRetriever|RAG|KnowledgeBase", path: "jarvis-rs")
// → Encontra: core/src/rag/retriever.rs, core/src/knowledge/

grep(pattern: "ProposalExecutor|GoalSystem", path: "jarvis-rs")
// → Encontra: daemon/src/executor.rs, daemon/src/main.rs
```

**Regra**: Confirme a existência de arquivos antes de dizer que estão faltando. Use padrões específicos do projeto.

### Fase 3: Leitura de Arquivos Principais

Leia os arquivos encontrados para entender a implementação:

**Arquivos-chave do Jarvis**:

```rust
// Core - Agent Loop
read_file(target_file: "jarvis-rs/core/src/agent_loop/mod.rs")
// → Entende: AgentLoop struct, run() method, Think→Execute→Observe loop

// Core - Planning
read_file(target_file: "jarvis-rs/core/src/agent/plan.rs")
// → Entende: PlanAgent trait, PlanAgentResult, RuleBasedPlanAgent

read_file(target_file: "jarvis-rs/core/src/autonomous/planner.rs")
// → Entende: ExecutionPlanner, ExecutionPlan, ExecutionStep

// Core - Memory/Context
read_file(target_file: "jarvis-rs/core/src/agent/session.rs")
// → Entende: AgentSession, AgentSessionManager, SessionContext

read_file(target_file: "jarvis-rs/core/src/context_manager/mod.rs")
// → Entende: ContextManager, history management

// Core - RAG
read_file(target_file: "jarvis-rs/core/src/rag/mod.rs")
// → Entende: Módulos RAG (indexer, retriever, store, embeddings)

// Core - Knowledge
read_file(target_file: "jarvis-rs/core/src/knowledge/mod.rs")
// → Entende: KnowledgeBase trait, implementations

// Daemon
read_file(target_file: "jarvis-rs/daemon/src/main.rs", offset: 1, limit: 100)
// → Entende: Entry point, goal system bootstrap

read_file(target_file: "jarvis-rs/daemon/src/executor.rs", offset: 1, limit: 100)
// → Entende: Proposal executor, action types

// TUI Integration
read_file(target_file: "jarvis-rs/tui/src/chatwidget.rs", offset: 755, limit: 30)
// → Entende: maybe_init_agent_loop(), roteamento

read_file(target_file: "jarvis-rs/tui/src/chatwidget/agent_loop_runner.rs", offset: 1, limit: 100)
// → Entende: Integração agent_loop com TUI
```

**Regra**: Leia pelo menos os arquivos `mod.rs` ou principais de cada módulo antes de analisar. Para arquivos grandes, use `offset` e `limit` para focar em seções relevantes.

### Fase 4: Verificação de Integração

Verifique se os componentes estão integrados ao fluxo principal:

**Verificações específicas do Jarvis**:

```rust
// Verificar uso do agent_loop no TUI
grep(pattern: "agent_loop|AgentLoop", path: "jarvis-rs/tui")
// → Deve encontrar: chatwidget.rs (maybe_init_agent_loop, agent_loop_tx)
// → Conclusão: ✅ Integrado ao TUI

// Verificar uso de planning
grep(pattern: "ExecutionPlanner|PlanAgent|create_plan", path: "jarvis-rs")
// → Se não encontrar em tui/ ou cli/: ❌ Não integrado ao fluxo principal
// → Se encontrar: ✅ Verificar se é chamado automaticamente

// Verificar uso de RAG
grep(pattern: "KnowledgeRetriever|RAG|rag", path: "jarvis-rs/tui")
// → Se não encontrar: ❌ RAG não integrado ao TUI
// → Se encontrar: ✅ Verificar onde é usado

// Verificar uso de daemon components
grep(pattern: "ProposalExecutor|execute_proposal", path: "jarvis-rs/daemon")
// → Deve encontrar: executor.rs
// → Verificar: Está sendo chamado pelo runner? ✅

// Verificar documentação de status (SEMPRE ler)
read_file(target_file: "docs/ANALISE_AUTONOMIA_JARVIS.md")
// → Linha 74-83: Tabela de status de integração
// → Mostra claramente: "Código existe | ❌ Não integrado"

read_file(target_file: "docs/architecture/autonomy-roadmap.md")
// → Roadmap e gaps G1-G6
```

**Regra**: Distinga entre "existe mas não está integrado" vs "não existe". A documentação em `docs/ANALISE_AUTONOMIA_JARVIS.md` tem uma tabela específica sobre isso.

---

## Checklist de Exploração

Antes de responder sobre autonomia/arquitetura, verifique:

### Componentes Core
- [ ] Agent Loop (`core/src/agent_loop/`)
- [ ] Planning Engine (`core/src/agent/plan.rs`, `core/src/autonomous/planner.rs`)
- [ ] Memory/Context (`core/src/agent/session.rs`, `core/src/context_manager/`)
- [ ] RAG System (`core/src/rag/`)
- [ ] Knowledge Base (`core/src/knowledge/`)
- [ ] Decision Engine (`core/src/autonomous/decision.rs`)

### Componentes Daemon
- [ ] Daemon (`daemon/src/`)
- [ ] Proposal Executor (`daemon/src/executor.rs`)
- [ ] Goal System (`daemon/src/main.rs`)
- [ ] Pipelines (`daemon/src/pipelines/`)

### Integração
- [ ] TUI usa agent_loop? (`tui/src/chatwidget.rs`)
- [ ] Daemon usa decision engine? (`daemon/src/decision_engine.rs`)
- [ ] RAG integrado ao TUI? (buscar em `tui/`)

### Documentação
- [ ] Ler `docs/ANALISE_AUTONOMIA_JARVIS.md`
- [ ] Ler `docs/architecture/autonomy-roadmap.md`
- [ ] Ler `docs/features/*.md` relevantes

---

## Padrões de Resposta

### ❌ ERRADO: Afirmar sem verificar

```
"O projeto não possui um motor de planejamento. Seria necessário implementar..."
```

### ✅ CORRETO: Verificar primeiro, depois analisar

```
"Busquei no codebase e encontrei:
- `core/src/agent/plan.rs` - PlanAgent implementado
- `core/src/autonomous/planner.rs` - ExecutionPlanner implementado

No entanto, verifiquei a integração e esses componentes não estão sendo chamados automaticamente no fluxo principal do TUI. O gap real é de integração, não de funcionalidade."
```

### ❌ ERRADO: Inferir de documentação genérica

```
"Baseado em AGENTS.md, o projeto parece ter..."
```

### ✅ CORRETO: Explorar código real

```
"Explorando o codebase, encontrei:
1. `core/src/agent_loop/mod.rs` - Agent loop implementado e funcional
2. `tui/src/chatwidget/agent_loop_runner.rs` - Integrado ao TUI
3. `scripts/dev-jarvis.bat` - Scripts de desenvolvimento usam agent_loop

O agent_loop está implementado e é usado quando modelos text-based são detectados."
```

---

## Exemplos de Análise Correta

### Exemplo 1: "O projeto tem agent loop?"

**Processo completo**:

1. **Busca semântica**:
   ```rust
   codebase_search("How does the agent loop work? What is the autonomous agent implementation?")
   ```
   → Encontra: `core/src/agent_loop/mod.rs`, `tui/src/chatwidget/agent_loop_runner.rs`

2. **Validação com busca direta**:
   ```rust
   glob_file_search("**/agent_loop*.rs")
   // → Encontra: core/src/agent_loop/mod.rs, tui/src/chatwidget/agent_loop_runner.rs
   
   grep("agent_loop|AgentLoop", "jarvis-rs/tui")
   // → Encontra: chatwidget.rs (múltiplas ocorrências)
   ```

3. **Leitura de arquivos principais**:
   ```rust
   read_file("jarvis-rs/core/src/agent_loop/mod.rs", offset: 1, limit: 100)
   // → Entende: AgentLoop struct, run() method, configuração
   
   read_file("jarvis-rs/tui/src/chatwidget.rs", offset: 755, limit: 30)
   // → Entende: maybe_init_agent_loop(), quando é chamado
   ```

4. **Verificação de integração**:
   ```rust
   grep("maybe_init_agent_loop|agent_loop_tx", "jarvis-rs/tui")
   // → Confirma: Integrado ao fluxo do TUI
   
   read_file("jarvis-rs/core/src/config/types.rs", offset: 1120, limit: 25)
   // → Entende: effective_mode(), detecção automática
   ```

5. **Resposta correta**:
   ```
   ✅ SIM, o projeto possui agent loop implementado em `core/src/agent_loop/`.
   
   Implementação:
   - `core/src/agent_loop/mod.rs` - AgentLoop struct com run() method
   - `tui/src/chatwidget/agent_loop_runner.rs` - Runner integrado ao TUI
   
   Integração:
   - `tui/src/chatwidget.rs:757` - maybe_init_agent_loop() é chamado na inicialização
   - `tui/src/chatwidget.rs:3517` - Mensagens são roteadas para agent_loop quando ativo
   - `core/src/config/types.rs:1124` - effective_mode() detecta automaticamente
   
   Uso:
   - Ativado automaticamente para modelos text-based (mistral-nemo, phi-3, etc.)
   - Pode ser forçado via config `agent_loop.mode = "text_based"`
   - Scripts dev-jarvis: `dev-jarvis.bat agent` força uso do agent_loop
   ```

### Exemplo 2: "O projeto tem sistema de planejamento?"

**Processo completo**:

1. **Busca semântica**:
   ```rust
   codebase_search("How does planning and task decomposition work?")
   ```
   → Encontra: `core/src/agent/plan.rs`, `core/src/autonomous/planner.rs`

2. **Validação**:
   ```rust
   glob_file_search("**/plan*.rs")
   // → Encontra: core/src/agent/plan.rs, core/src/autonomous/planner.rs
   
   grep("PlanAgent|ExecutionPlanner", "jarvis-rs")
   // → Encontra: Definições nos arquivos acima
   ```

3. **Leitura**:
   ```rust
   read_file("jarvis-rs/core/src/agent/plan.rs", offset: 1, limit: 100)
   // → Entende: PlanAgent trait, PlanAgentResult
   
   read_file("jarvis-rs/core/src/autonomous/planner.rs", offset: 1, limit: 60)
   // → Entende: ExecutionPlanner, ExecutionPlan
   ```

4. **Verificação de integração**:
   ```rust
   grep("PlanAgent|create_plan|ExecutionPlanner", "jarvis-rs/tui")
   // → Se não encontrar: ❌ Não integrado ao TUI
   
   grep("PlanAgent|create_plan|ExecutionPlanner", "jarvis-rs/cli")
   // → Se não encontrar: ❌ Não integrado ao CLI
   
   read_file("docs/ANALISE_AUTONOMIA_JARVIS.md", offset: 74, limit: 10)
   // → Confirma: "Código existe | ❌ Não integrado"
   ```

5. **Resposta correta**:
   ```
   ✅ SIM, o projeto possui sistemas de planejamento, mas NÃO estão integrados ao fluxo principal.
   
   Implementações encontradas:
   - `core/src/agent/plan.rs` - PlanAgent trait, RuleBasedPlanAgent
   - `core/src/autonomous/planner.rs` - ExecutionPlanner, ExecutionPlan
   
   Status de integração:
   - ❌ Não integrado ao TUI (não encontrado em chatwidget.rs)
   - ❌ Não integrado ao CLI (não encontrado em cli/)
   - ✅ Código existe com testes unitários
   
   Gap: O gap é de integração, não de funcionalidade. Os componentes existem mas não são chamados automaticamente.
   ```

---

## Ferramentas Disponíveis

### codebase_search
- **Uso**: Busca semântica por conceitos
- **Quando**: Fase inicial de exploração
- **Exemplo**: "How does X work?", "Where is Y implemented?"

### grep
- **Uso**: Busca exata por padrões de texto
- **Quando**: Validação e verificação de uso
- **Exemplo**: `grep("AgentLoop", "jarvis-rs")`

### glob_file_search
- **Uso**: Busca por padrões de arquivo
- **Quando**: Encontrar módulos relacionados
- **Exemplo**: `glob_file_search("**/agent*.rs")`

### read_file
- **Uso**: Ler conteúdo de arquivos
- **Quando**: Entender implementação específica
- **Exemplo**: `read_file("jarvis-rs/core/src/agent_loop/mod.rs")`

---

## Regras de Ouro

1. **Nunca afirme que algo não existe sem buscar primeiro**
2. **Sempre distinga entre "não existe" vs "não integrado"**
3. **Mencione componentes encontrados, mesmo que não integrados**
4. **Base análises em código, não apenas em documentação**
5. **Cite arquivos específicos quando possível**

---

## Mapeamento Rápido de Componentes

### Componentes Core (jarvis-rs/core/src/)

| Componente | Arquivo Principal | Status | Integrado? |
|------------|-------------------|--------|------------|
| Agent Loop | `agent_loop/mod.rs` | ✅ Implementado | ✅ Sim (TUI) |
| Planning (Agent) | `agent/plan.rs` | ✅ Implementado | ❌ Não |
| Planning (Autonomous) | `autonomous/planner.rs` | ✅ Implementado | ❌ Não |
| Memory/Context | `agent/session.rs` | ✅ Implementado | ⚠️ Parcial |
| Context Manager | `context_manager/` | ✅ Implementado | ✅ Sim (TUI) |
| RAG System | `rag/mod.rs` | ✅ Implementado | ❌ Não |
| Knowledge Base | `knowledge/mod.rs` | ✅ Implementado | ❌ Não |
| Decision Engine | `autonomous/decision.rs` | ✅ Implementado | ⚠️ Parcial (daemon) |
| Intent Detection | `intent/mod.rs` | ✅ Implementado | ❌ Não |
| Skills System | `skills/mod.rs` | ✅ Implementado | ❌ Não |

### Componentes Daemon (jarvis-rs/daemon/src/)

| Componente | Arquivo Principal | Status | Integrado? |
|------------|-------------------|--------|------------|
| Daemon Main | `main.rs` | ✅ Implementado | ✅ Sim |
| Proposal Executor | `executor.rs` | ✅ Implementado | ✅ Sim |
| Decision Engine | `decision_engine.rs` | ✅ Implementado | ✅ Sim |
| Goal System | `main.rs` (bootstrap) | ✅ Implementado | ✅ Sim |
| Pipelines | `pipelines/` | ✅ Implementado | ✅ Sim |
| Scheduler | `scheduler.rs` | ✅ Implementado | ✅ Sim |

### Integrações TUI (jarvis-rs/tui/src/)

| Componente | Arquivo | Status |
|------------|---------|--------|
| Agent Loop Integration | `chatwidget.rs:757` | ✅ Integrado |
| Agent Loop Runner | `chatwidget/agent_loop_runner.rs` | ✅ Integrado |
| Roteamento | `chatwidget.rs:3517` | ✅ Funcional |

## Referências Essenciais

### Documentação de Status
- `docs/ANALISE_AUTONOMIA_JARVIS.md` - **LEIA PRIMEIRO**: Análise real do estado atual, tabela de integração linha 74-83
- `docs/architecture/autonomy-roadmap.md` - Roadmap de autonomia, gaps G1-G6
- `docs/reports/gap-analysis-response-vs-reality.md` - Relatório comparativo (resposta genérica vs realidade)

### Documentação de Features
- `docs/features/agentic-loop.md` - Especificação do agent loop
- `docs/features/planning-engine.md` - Especificação do planning engine
- `docs/features/rag-context-management.md` - Especificação do RAG
- `docs/features/daemon-automation.md` - Especificação do daemon

### Código de Referência
- `jarvis-rs/core/src/lib.rs` - Exports principais do core
- `jarvis-rs/Cargo.toml` - Estrutura do workspace
- `AGENTS.md` - Convenções de código (mas NÃO confie apenas nisso para análises)

---

**Última atualização**: 2026-02-18  
**Versão**: 2.0 (aplicado com exemplos práticos do projeto Jarvis)
