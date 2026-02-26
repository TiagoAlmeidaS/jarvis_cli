# Troubleshooting: Agente Não Usa Ferramentas (Tools)

**Data**: 2026-02-18  
**Problema**: Agente Jarvis não está usando ferramentas disponíveis (`codebase_search`, `grep`, `read_file`, etc.) e tenta usar comandos de shell em vez disso.

---

## Sintomas Observados

1. **Agente tenta usar comandos de shell**:
   ```
   "ls -F"
   "Poderia me confirmar que o diretório jarvis-rs está localizado em..."
   ```

2. **Agente não usa ferramentas disponíveis**:
   - Não usa `codebase_search` para busca semântica
   - Não usa `grep` para busca de padrões
   - Não usa `read_file` para ler arquivos
   - Não usa `list_dir` para explorar estrutura

3. **Respostas genéricas**:
   - Pede confirmação do usuário em vez de explorar
   - Não segue o processo documentado em `AGENTS.md`

---

## Possíveis Causas

### Causa 1: Modo Text-Based sem Tool Calling

**Problema**: O agente está usando um modelo text-based (como `mistral-nemo`) que não suporta function calling nativo, mas o agent_loop não está ativo ou não está parseando corretamente as tool calls.

**Verificação**:
```bash
# Verificar qual modelo está sendo usado
# Verificar se agent_loop está ativo
# Verificar logs do TUI para tool calls
```

**Solução**:
- Usar modelo com function calling nativo (GPT-4, Claude, Qwen)
- Ou garantir que agent_loop está ativo e funcionando
- Verificar `config.toml` para `agent_loop.mode`

### Causa 2: Ferramentas Não Disponíveis no Contexto

**Problema**: As ferramentas não estão sendo registradas ou expostas ao modelo.

**Verificação**:
- Verificar se o modelo tem acesso às tool definitions
- Verificar se o tool registry está populado
- Verificar logs de inicialização

**Solução**:
- Verificar configuração de tools em `core/src/tools/spec.rs`
- Verificar se features estão habilitadas
- Verificar se MCP servers estão configurados

### Causa 3: Instruções Não Sendo Seguidas

**Problema**: O agente não está seguindo as instruções em `AGENTS.md` sobre exploração do codebase.

**Verificação**:
- Verificar se `AGENTS.md` está sendo lido
- Verificar se as instruções estão claras o suficiente
- Verificar se o modelo está processando as instruções

**Solução**:
- Melhorar instruções em `AGENTS.md` (já feito)
- Adicionar exemplos mais explícitos
- Usar prompt injection mais forte

### Causa 4: Modelo Não Entende Tool Calling

**Problema**: O modelo não está gerando tool calls no formato esperado.

**Verificação**:
- Verificar formato das tool calls geradas
- Verificar se o modelo suporta function calling
- Verificar logs de parsing

**Solução**:
- Usar modelo com suporte robusto a function calling
- Verificar configuração de tool calling mode
- Verificar text-based tool calling se necessário

---

## Soluções Recomendadas

### Solução 1: Usar Modelo com Function Calling Nativo

**Para desenvolvimento/testes**:
```bash
# Usar modelo premium com function calling
dev-jarvis.bat qwen
# ou
dev-jarvis.bat free_google  # Gemini tem function calling
```

**Configuração**:
```toml
# config.toml
[model]
provider = "openrouter"  # ou "google", "openai"
name = "qwen/qwen3-coder-next"  # ou modelo com function calling
```

### Solução 2: Garantir Agent Loop Ativo (Text-Based)

**Se usando modelo text-based**:
```bash
# Forçar agent loop
dev-jarvis.bat agent
```

**Configuração**:
```toml
# config.toml
[agent_loop]
mode = "text_based"  # ou "auto"
```

**Verificar**:
- Logs devem mostrar: "Agent loop enabled for text-based model"
- `tui/src/chatwidget.rs:757` deve chamar `maybe_init_agent_loop()`

### Solução 3: Melhorar Instruções no Prompt

**Adicionar ao prompt do modelo**:
```
CRITICAL: You MUST use available tools to explore the codebase.

When asked to analyze:
1. IMMEDIATELY use codebase_search() to find relevant code
2. IMMEDIATELY use grep() to validate findings
3. IMMEDIATELY use read_file() to read implementation files

DO NOT ask for confirmation. DO NOT use shell commands. USE THE TOOLS.
```

**Localização**: `jarvis-rs/core/prompt.md` ou `jarvis-rs/core/templates/`

### Solução 4: Verificar Tool Registry

**Verificar se tools estão registradas**:
```rust
// core/src/tools/spec.rs
// Verificar se codebase_search, grep, read_file estão no registry
```

**Verificar features**:
```toml
# config.toml
[features]
# Verificar se features necessárias estão habilitadas
```

---

## Diagnóstico Passo a Passo

### 1. Verificar Modelo em Uso

```bash
# Ver logs do TUI ao iniciar
# Procurar por: "Using model: ..."
# Procurar por: "Agent loop enabled" ou "Using Responses API"
```

### 2. Verificar Tool Calling Mode

```bash
# Verificar config
cat ~/.jarvis/config.toml | grep agent_loop

# Verificar logs
# Procurar por: "effective_mode" ou "ToolCallingMode"
```

### 3. Verificar Tool Calls Geradas

```bash
# Ver logs do TUI durante interação
# Procurar por: "tool_call" ou "function_call"
# Verificar se estão no formato correto
```

### 4. Testar Tool Calling Manualmente

```bash
# Testar com modelo conhecido por funcionar
dev-jarvis.bat qwen

# Pedir: "Liste os arquivos em jarvis-rs/core/src"
# Deve usar list_directory tool, não shell
```

---

## Exemplo de Comportamento Correto

### ❌ ERRADO (Comportamento Atual)

```
Usuário: "Analise o projeto"
Agente: "Vou listar o diretório: ls -F"
Agente: "Poderia me confirmar que o diretório jarvis-rs está localizado em..."
```

### ✅ CORRETO (Comportamento Esperado)

```
Usuário: "Analise o projeto"
Agente: [Usa codebase_search("What is the main architecture of jarvis-rs?")]
Agente: [Usa list_dir("jarvis-rs")]
Agente: [Usa read_file("jarvis-rs/core/src/lib.rs")]
Agente: "Analisando o projeto, encontrei:
- Core em jarvis-rs/core/
- Daemon em jarvis-rs/daemon/
- TUI em jarvis-rs/tui/
..."
```

---

## Checklist de Verificação

Antes de reportar problema, verificar:

- [ ] Modelo suporta function calling? (GPT-4, Claude, Qwen, Gemini)
- [ ] Agent loop está ativo se usando modelo text-based?
- [ ] Tools estão registradas? (verificar logs)
- [ ] AGENTS.md está sendo lido? (verificar prompt)
- [ ] Tool calls estão sendo geradas? (verificar logs)
- [ ] Tool calls estão no formato correto? (verificar parsing)

---

## Referências

- `AGENTS.md` - Instruções de exploração do codebase
- `docs/reports/codebase-exploration-guide.md` - Guia detalhado
- `jarvis-rs/core/src/tools/text_tool_calling.rs` - Text-based tool calling
- `jarvis-rs/core/src/tools/spec.rs` - Tool registry
- `docs/features/tool-calling-native.md` - Tool calling nativo
- `docs/features/agentic-loop.md` - Agent loop

---

**Última atualização**: 2026-02-18
