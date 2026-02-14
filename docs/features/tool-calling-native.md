# Tool Calling Nativo — Client-Side Tool Execution

**Status**: Implementado
**Prioridade**: ALTA
**Gap**: G4
**Roadmap**: Fase 2, Step 2.1
**Ultima atualizacao**: 2026-02-13

---

## 1. Problema

Hoje o TUI do Jarvis depende do **modelo LLM** para decidir quando e como chamar
tools (shell, file edit, web search). Isso funciona bem com modelos premium
(GPT-4, Claude) mas **falha com modelos baratos** como `mistral-nemo` que nao
suportam function calling de forma robusta.

### Consequencia

- Modelos baratos retornam resposta vazia para tarefas que precisam de tools
- O usuario e forcado a usar modelos caros para tarefas simples
- O Jarvis e tao inteligente quanto o modelo — sem inteligencia propria

### Como Claude Code / Cursor resolvem

Eles usam **tool calling client-side**: o client define as tools disponiveis,
o modelo retorna um JSON com `tool_use`, o **client executa** a tool, captura
o resultado, e devolve ao modelo para continuar. O loop e gerenciado pelo
client, nao pelo modelo.

---

## 2. Solucao: Tool Registry + Client-Side Dispatcher

### Arquitetura proposta

```
┌─────────────────────────────────────────────────────────┐
│                     JARVIS TUI / CLI                     │
│                                                          │
│  ┌──────────────┐     ┌──────────────┐                  │
│  │ Tool Registry│     │ Tool         │                  │
│  │              │     │ Dispatcher   │                  │
│  │ - Shell      │     │              │                  │
│  │ - FileRead   │     │ 1. Parse     │                  │
│  │ - FileWrite  │     │    tool_use  │                  │
│  │ - FileEdit   │     │ 2. Validate  │                  │
│  │ - WebSearch  │     │ 3. Execute   │                  │
│  │ - ListDir    │     │ 4. Capture   │                  │
│  │ - Grep       │     │    output    │                  │
│  │ - GitOps     │     │ 5. Return to │                  │
│  │ - Custom...  │     │    model     │                  │
│  └──────┬───────┘     └──────┬───────┘                  │
│         │                     │                          │
│         └─────────┬───────────┘                          │
│                   │                                      │
│          ┌────────▼────────┐                             │
│          │   LLM Client    │                             │
│          │                 │                             │
│          │ Messages:       │                             │
│          │ [system, user,  │                             │
│          │  assistant,     │──── API ───▶ OpenRouter /   │
│          │  tool_result,   │              Ollama / etc   │
│          │  assistant...]  │                             │
│          └─────────────────┘                             │
└─────────────────────────────────────────────────────────┘
```

### Fluxo detalhado

```
1. Usuario envia mensagem: "Analise o projeto jarvis_cli"

2. Jarvis envia ao LLM com tools definitions:
   {
     "messages": [...],
     "tools": [
       {"type": "function", "function": {"name": "shell", "parameters": {...}}},
       {"type": "function", "function": {"name": "read_file", "parameters": {...}}},
       {"type": "function", "function": {"name": "list_directory", "parameters": {...}}},
       ...
     ]
   }

3. LLM responde com tool_use (mesmo modelos baratos suportam isso no formato basico):
   {
     "role": "assistant",
     "tool_calls": [
       {"id": "call_1", "function": {"name": "list_directory", "arguments": "{\"path\": \".\"}"}}
     ]
   }

4. Jarvis (client-side) executa a tool:
   - Valida permissoes (sandbox policy)
   - Executa: list_directory(".")
   - Captura resultado: ["jarvis-rs/", "docs/", "scripts/", ...]

5. Jarvis envia resultado de volta ao LLM:
   {
     "role": "tool",
     "tool_call_id": "call_1",
     "content": "jarvis-rs/\ndocs/\nscripts/\n..."
   }

6. LLM continua (pode chamar mais tools ou responder ao usuario)

7. Repete ate o LLM responder sem tool_calls (fim do turno)
```

---

## 3. Tool Registry

### 3.1 Trait

```rust
/// Represents a tool that can be invoked by the LLM.
#[async_trait]
pub trait Tool: Send + Sync {
    /// Tool name (used in function calling).
    fn name(&self) -> &str;

    /// Human-readable description for the LLM.
    fn description(&self) -> &str;

    /// JSON Schema for the tool's parameters.
    fn parameters_schema(&self) -> serde_json::Value;

    /// Execute the tool with the given arguments.
    async fn execute(&self, args: serde_json::Value, ctx: &ToolContext) -> Result<ToolResult>;

    /// Whether this tool requires user approval.
    fn requires_approval(&self) -> bool { true }

    /// Risk level (affects approval policy).
    fn risk_level(&self) -> ToolRiskLevel { ToolRiskLevel::Medium }
}

pub struct ToolContext {
    pub cwd: PathBuf,
    pub sandbox_policy: SandboxPolicy,
    pub approval_policy: AskForApproval,
    pub writable_roots: Vec<PathBuf>,
}

pub struct ToolResult {
    pub output: String,
    pub success: bool,
    pub truncated: bool,
}

pub enum ToolRiskLevel {
    /// Read-only, never needs approval (list_dir, read_file, grep)
    Safe,
    /// May modify files in workspace (file_edit, write_file)
    Medium,
    /// Can execute arbitrary code or access network (shell, web_search)
    High,
}
```

### 3.2 Built-in Tools

| Tool | Descricao | Risk | Approval |
|------|-----------|------|----------|
| `read_file` | Le conteudo de um arquivo | Safe | Never |
| `list_directory` | Lista arquivos de um diretorio | Safe | Never |
| `grep_search` | Busca por pattern em arquivos | Safe | Never |
| `write_file` | Escreve/cria arquivo | Medium | Policy-dependent |
| `edit_file` | Edita trecho de arquivo (str_replace) | Medium | Policy-dependent |
| `shell` | Executa comando shell | High | Policy-dependent |
| `web_search` | Busca na web | Medium | Policy-dependent |
| `git_status` | Status do repositorio | Safe | Never |
| `git_diff` | Diff de mudancas | Safe | Never |
| `git_log` | Historico de commits | Safe | Never |

### 3.3 Registry

```rust
pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new_with_defaults() -> Self {
        let mut registry = Self::new();
        registry.register(Arc::new(ReadFileTool));
        registry.register(Arc::new(ListDirectoryTool));
        registry.register(Arc::new(GrepSearchTool));
        registry.register(Arc::new(WriteFileTool));
        registry.register(Arc::new(EditFileTool));
        registry.register(Arc::new(ShellTool));
        registry.register(Arc::new(GitStatusTool));
        // ...
        registry
    }

    /// Generate the "tools" array for the LLM API request.
    pub fn to_api_tools(&self) -> Vec<serde_json::Value> {
        self.tools.values().map(|tool| {
            serde_json::json!({
                "type": "function",
                "function": {
                    "name": tool.name(),
                    "description": tool.description(),
                    "parameters": tool.parameters_schema(),
                }
            })
        }).collect()
    }
}
```

---

## 4. Compatibilidade com modelos

### 4.1 Modelos com function calling nativo

Modelos que suportam `tools` no request body (maioria dos modelos OpenRouter):
- OpenAI GPT-4o, GPT-4o-mini
- Anthropic Claude 3+
- Qwen 3 Coder
- DeepSeek V3/R1
- Mistral Large

Para esses, usar o formato padrao OpenAI `tools` + `tool_calls`.

### 4.2 Modelos SEM function calling

Para modelos como `mistral-nemo` que nao suportam `tools`:

**Estrategia**: Injetar as tools no **system prompt** como texto e parsear
a resposta do modelo para extrair chamadas de tool.

```
System prompt injection:
You have access to the following tools. To use a tool, respond with a JSON block:
```tool_call
{"name": "read_file", "arguments": {"path": "src/main.rs"}}
```

Available tools:
- read_file(path: string): Read file contents
- list_directory(path: string): List directory
- shell(command: string): Execute shell command
...

After receiving tool results, continue your analysis.
```

O client parseia blocos ` ```tool_call ``` ` da resposta do modelo e os executa.

### 4.3 Deteccao automatica

```rust
pub enum ToolCallingMode {
    /// Model supports native function calling (OpenAI format).
    Native,
    /// Model uses text-based tool calling (prompt injection).
    TextBased,
    /// No tool calling support.
    None,
}

impl ToolCallingMode {
    pub fn detect(model: &str, provider: &str) -> Self {
        // Known models with native support
        if model.contains("gpt-4") || model.contains("claude")
            || model.contains("qwen") || model.contains("deepseek")
        {
            return Self::Native;
        }
        // Known models that work with text-based
        if model.contains("mistral") || model.contains("llama") || model.contains("gemma") {
            return Self::TextBased;
        }
        // Default: try native first, fallback to text-based
        Self::Native
    }
}
```

---

## 5. Impacto

| Antes | Depois |
|-------|--------|
| Modelos baratos retornam vazio para tasks complexas | Qualquer modelo funciona com tools |
| Tool calling depende do modelo | Client gerencia tools independente do modelo |
| Sem inteligencia propria | Jarvis tem tool registry proprio |
| Limitado a modelos premium | Modelos baratos ($0.02/M) fazem tool calling |

---

## 6. Testes

| Teste | Tipo | Descricao |
|-------|------|-----------|
| `test_tool_registry_api_format` | Unitario | Gera JSON tools correto |
| `test_tool_dispatch` | Unitario | Dispatch correto por nome |
| `test_read_file_tool` | Unitario | Le arquivo e retorna conteudo |
| `test_shell_tool_sandbox` | Unitario | Shell respeita sandbox policy |
| `test_text_based_parsing` | Unitario | Parseia tool_call de texto |
| `test_native_tool_call_round_trip` | Integracao | LLM chama tool, client executa, retorna |
| `test_approval_required` | Unitario | Tool high-risk pede aprovacao |
| `test_safe_tool_no_approval` | Unitario | Tool safe executa sem pedir |

---

## 7. Arquivos a criar/modificar

| Arquivo | Acao | Descricao |
|---------|------|-----------|
| `core/src/tools/mod.rs` | **Criar** | Trait Tool + ToolRegistry + ToolContext |
| `core/src/tools/builtin/mod.rs` | **Criar** | Modulo para tools built-in |
| `core/src/tools/builtin/read_file.rs` | **Criar** | ReadFileTool |
| `core/src/tools/builtin/list_dir.rs` | **Criar** | ListDirectoryTool |
| `core/src/tools/builtin/grep.rs` | **Criar** | GrepSearchTool |
| `core/src/tools/builtin/shell.rs` | **Criar** | ShellTool |
| `core/src/tools/builtin/edit_file.rs` | **Criar** | EditFileTool |
| `core/src/tools/builtin/write_file.rs` | **Criar** | WriteFileTool |
| `core/src/tools/dispatcher.rs` | **Criar** | ToolDispatcher (executa + captura) |
| `core/src/tools/text_parser.rs` | **Criar** | Parser para text-based tool calls |
| `tui/src/chatwidget.rs` | Modificar | Integrar ToolDispatcher no turn loop |

---

## 8. Estimativa

- **Complexidade**: Alta
- **Tempo estimado**: 3-5 dias
- **Risco**: Medio (precisa testar com varios modelos)
- **Prerequisito**: Nenhum
- **Dependente dele**: Agentic Loop (G5)
