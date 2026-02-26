# Análise: Configuração do Agent Loop no config.toml

**Data**: 2026-02-18  
**Arquivo analisado**: `.github/codex/home/config.toml`  
**Modelo em uso no TUI**: `gemini-2.5-flash` (Google AI Studio)  
**Nota**: O config.toml mostra `gpt-5.1`, mas o TUI está usando `gemini-2.5-flash`

---

## Estado Atual da Configuração

### Configuração Atual
```toml
model = "gpt-5.1"

[github]
pat_secret_name = "GITHUB_PAT"
api_base_url = "https://api.github.com"
```

**Observação**: A seção `[agent_loop]` **não está presente** no arquivo.

---

## Análise da Configuração

### 1. Modelo em Uso: `gpt-5.1`

**Características**:
- Modelo premium da OpenAI
- **Suporta function calling nativo** via Responses API
- Não precisa de agent loop (text-based)

**Comportamento esperado**:
- Com `agent_loop.mode = "auto"` (default): Usa **Native** (Responses API)
- Com `agent_loop.mode = "native"`: Usa **Native** (explicitamente)
- Com `agent_loop.mode = "text_based"`: Força agent loop (não recomendado para GPT-5.1)

### 2. Configuração Atual vs Recomendada

#### ❌ Configuração Atual (Faltando)
```toml
# agent_loop não está configurado
# Usa defaults: mode = "auto"
```

#### ✅ Configuração Recomendada para GPT-5.1

**Opção 1: Deixar padrão (recomendado)**
```toml
model = "gpt-5.1"

# agent_loop não precisa ser configurado
# Default "auto" detecta que GPT-5.1 tem function calling nativo
# Resultado: Usa Responses API (nativo)
```

**Opção 2: Configuração explícita (opcional)**
```toml
model = "gpt-5.1"

[agent_loop]
mode = "native"  # Força uso de function calling nativo
# Outros campos não são necessários para modelos premium
```

**Opção 3: Se quiser testar agent loop (não recomendado)**
```toml
model = "gpt-5.1"

[agent_loop]
mode = "text_based"  # Força agent loop mesmo para GPT-5.1
base_url = "https://api.openai.com/v1"
api_key = "env:OPENAI_API_KEY"
model = "gpt-5.1"  # Mesmo modelo, mas via agent loop
```

---

## Valores Válidos para `agent_loop.mode`

### Enum `AgentLoopMode`

| Valor | Descrição | Quando Usar |
|-------|-----------|-------------|
| `"auto"` | **Default** - Detecta automaticamente | ✅ **Recomendado** - Funciona para todos os modelos |
| `"native"` | Força function calling nativo | Modelos premium (GPT-4, Claude, Qwen) |
| `"text_based"` | Força agent loop (text-based) | Modelos baratos (mistral-nemo, phi-3, Ollama local) |
| `"disabled"` | Desabilita tool calling | Apenas para testes/debug |

### Detecção Automática (`mode = "auto"`)

O sistema detecta automaticamente modelos text-based baseado em padrões:

**Modelos detectados como Text-Based** (precisam agent loop):
- `mistral-nemo`
- `phi-3`, `phi-2`
- `tinyllama`
- `stablelm`
- `gemma-2b`, `gemma-7b`
- `yi-6b`, `yi-9b`

**Modelos detectados como Native** (usam Responses API):
- `gpt-5.1`, `gpt-4o`, `gpt-4`, etc.
- `claude-3`, `claude-4`, etc.
- `qwen3-coder-next`, etc.
- Qualquer outro não listado acima

---

## Configuração Completa do Agent Loop

### Estrutura Completa (para referência)

```toml
[agent_loop]
# Modo de tool calling
mode = "auto"  # "auto" | "text_based" | "native" | "disabled"

# Configuração da API LLM (apenas para text_based)
base_url = "http://localhost:11434/v1"  # Default: Ollama local
api_key = "env:OPENROUTER_API_KEY"      # ou valor direto, ou "env:VAR_NAME"
api_key_env = "OPENROUTER_API_KEY"      # Alternativa a api_key = "env:..."

# Modelo (opcional, usa model principal se não especificado)
model = "mistral-nemo"

# Parâmetros do LLM
temperature = 0.7          # Default: 0.7
max_tokens = 4096          # Default: 4096

# Limites do loop
max_iterations = 25        # Default: 25 (Think→Execute→Observe)
timeout_sec = 300          # Default: 300 (5 minutos)
max_context_tokens = 32000 # Default: 32000 (antes de compactar)
```

### Valores Default

Se `[agent_loop]` não estiver configurado, os defaults são:

```rust
mode: "auto"
base_url: "http://localhost:11434/v1"  # Ollama local
api_key: ""  # Auto-detecta baseado em base_url
model: None  # Usa model principal
temperature: 0.7
max_tokens: 4096
max_iterations: 25
timeout_sec: 300
max_context_tokens: 32000
```

---

## Recomendações para o Config Atual

### Para `gpt-5.1` (Modelo Premium)

**Recomendação**: **Não configurar `[agent_loop]`** ou usar `mode = "native"`

**Motivo**:
- GPT-5.1 tem function calling nativo robusto
- Responses API é mais eficiente para modelos premium
- Agent loop é desnecessário e pode adicionar latência

**Configuração recomendada**:
```toml
model = "gpt-5.1"

# agent_loop não precisa ser configurado
# O default "auto" detecta que GPT-5.1 usa Native
```

### Se Mudar para Modelo Text-Based

**Exemplo: Usar mistral-nemo via OpenRouter**

```toml
model = "mistralai/mistral-nemo"

[agent_loop]
mode = "auto"  # Auto detecta que mistral-nemo precisa de text_based
base_url = "https://openrouter.ai/api/v1"
api_key = "env:OPENROUTER_API_KEY"
# model não precisa ser especificado (usa model principal)
```

**Exemplo: Usar Ollama local**

```toml
model = "llama3.2:3b"

[agent_loop]
mode = "text_based"  # Força text_based para Ollama
base_url = "http://localhost:11434/v1"
# api_key não é necessário para Ollama local
```

---

## Resolução de API Key

O sistema resolve a API key na seguinte ordem de prioridade:

1. **`api_key_env`** (maior prioridade)
2. **`api_key = "env:VAR_NAME"`** (sintaxe env:)
3. **`api_key = "valor_direto"`** (valor literal)
4. **Auto-detecta baseado em `base_url`**:
   - `openrouter.ai` → `OPENROUTER_API_KEY`
   - `api.openai.com` → `OPENAI_API_KEY`
   - `generativelanguage.googleapis.com` → `GOOGLE_API_KEY`
   - `anthropic.com` → `ANTHROPIC_API_KEY`
   - `localhost:11434` → "" (sem key, Ollama local)

---

## Verificação de Configuração

### Como Verificar se Agent Loop Está Ativo

**Logs do TUI ao iniciar**:
```
Agent loop enabled for text-based model: mistral-nemo
```
ou
```
Using Responses API (native function calling)
```

**Código de verificação**:
```rust
// tui/src/chatwidget.rs:757
let effective = self.config.agent_loop.effective_mode(model_name);
if effective == AgentLoopMode::TextBased {
    // Agent loop será usado
}
```

---

## Conclusão

### Para o Config Atual (gpt-5.1)

✅ **Configuração atual está correta**: Não precisa de `[agent_loop]`

**Comportamento esperado**:
- `mode = "auto"` (default)
- `effective_mode("gpt-5.1")` → `Native`
- Usa Responses API (function calling nativo)
- **Não usa agent loop** (correto para GPT-5.1)

### Se o Problema Persistir (Agente Não Usa Tools)

O problema **não é** a configuração do agent_loop (está correta para GPT-5.1).

Possíveis causas:
1. **Modelo não está recebendo tool definitions** → Verificar tool registry
2. **Instruções não estão sendo seguidas** → Verificar prompt/AGENTS.md
3. **Tool calls não estão sendo geradas** → Verificar logs do modelo

Ver: `docs/reports/troubleshooting-agent-tools.md`

---

## Referências

- `jarvis-rs/core/src/config/types.rs:936` - Enum AgentLoopMode
- `jarvis-rs/core/src/config/types.rs:1124` - effective_mode()
- `jarvis-rs/core/src/tools/text_tool_calling.rs:57` - is_text_based_model()
- `docs/features/agentic-loop.md` - Documentação do agent loop
- `docs/reports/troubleshooting-agent-tools.md` - Troubleshooting

---

**Última atualização**: 2026-02-18
