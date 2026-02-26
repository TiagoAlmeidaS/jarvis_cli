# Análise: Gemini 2.5 Flash e Tool Calling

**Data**: 2026-02-18  
**Problema**: Agente usando `gemini-2.5-flash` não está usando ferramentas  
**Modelo em uso**: `gemini-2.5-flash` (Google AI Studio)

---

## Descoberta Crítica

O modelo em uso é **`gemini-2.5-flash`**, não `gpt-5.1` como estava no config.toml.

**Tela do TUI mostra**:
```
model: gemini-2.5-flash
```

Isso pode ser a causa raiz do problema!

---

## Análise do Gemini 2.5 Flash

### 1. Detecção de Modo

**Código de detecção** (`jarvis-rs/core/src/tools/text_tool_calling.rs:57`):
```rust
fn is_text_based_model(model_lower: &str) -> bool {
    let text_only_patterns = [
        "mistral-nemo",
        "phi-3",
        "phi-2",
        "tinyllama",
        "stablelm",
        "gemma-2b",
        "gemma-7b",
        "yi-6b",
        "yi-9b",
    ];
    // ...
}
```

**Resultado para `gemini-2.5-flash`**:
- ❌ **NÃO está na lista** de modelos text-based
- ✅ **Detectado como Native** (default)
- ✅ **Usa Responses API** (function calling nativo)

### 2. Configuração do Provider Google

**Código** (`jarvis-rs/core/src/model_provider_info.rs:515`):
```rust
pub fn create_google_provider() -> ModelProviderInfo {
    ModelProviderInfo {
        base_url: "https://generativelanguage.googleapis.com/v1beta/openai",
        wire_api: WireApi::Responses,  // ✅ Usa Responses API
        uses_chat_completions_api: true, // ✅ Chat completions
        // ...
    }
}
```

**Características**:
- ✅ Usa `WireApi::Responses` (suporta tools)
- ✅ Usa chat completions API
- ✅ Endpoint OpenAI-compatible (`/v1beta/openai`)

### 3. Comportamento Esperado

Com `agent_loop.mode = "auto"` (default):
- `effective_mode("gemini-2.5-flash")` → `Native`
- Deve usar Responses API com function calling nativo
- **NÃO deve usar agent loop**

---

## Possíveis Problemas com Gemini

### Problema 1: Gemini Pode Não Gerar Tool Calls Corretamente

**Hipótese**: Gemini 2.5 Flash pode ter problemas com function calling mesmo via Responses API.

**Evidência**:
- Google Gemini usa endpoint `/v1beta/openai` (compatibilidade OpenAI)
- Pode haver diferenças sutis na implementação
- Modelos "flash" podem ter limitações

**Solução**: Forçar agent loop (text-based) para Gemini

### Problema 2: Endpoint `/v1beta/openai` Pode Não Suportar Tools

**Hipótese**: O endpoint de compatibilidade OpenAI pode não suportar `tools` parameter corretamente.

**Verificação necessária**:
- Verificar se Google AI Studio suporta `tools` no request
- Verificar se há diferenças na API

**Solução**: Usar agent loop se endpoint não suportar

### Problema 3: Modelo Não Está Sendo Instruído Corretamente

**Hipótese**: O prompt não está instruindo o modelo a usar tools.

**Solução**: Melhorar instruções (já feito em AGENTS.md)

---

## Soluções Recomendadas

### Solução 1: Forçar Agent Loop para Gemini (RECOMENDADO)

**Configuração**:
```toml
model = "gemini-2.5-flash"

[agent_loop]
mode = "text_based"  # Força agent loop mesmo para Gemini
base_url = "https://generativelanguage.googleapis.com/v1beta/openai"
api_key = "env:GOOGLE_API_KEY"
model = "gemini-2.5-flash"
```

**Vantagens**:
- ✅ Garante que tools serão usadas (via text-based parsing)
- ✅ Funciona mesmo se Gemini não gerar tool_calls nativos
- ✅ Mais confiável para modelos "flash"

**Desvantagens**:
- ⚠️ Pode ser mais lento (múltiplas iterações)
- ⚠️ Usa mais tokens (prompt injection)

### Solução 2: Adicionar Gemini à Lista de Text-Based

**Modificação no código**:
```rust
// jarvis-rs/core/src/tools/text_tool_calling.rs:57
fn is_text_based_model(model_lower: &str) -> bool {
    let text_only_patterns = [
        "mistral-nemo",
        "phi-3",
        "phi-2",
        "tinyllama",
        "stablelm",
        "gemma-2b",
        "gemma-7b",
        "yi-6b",
        "yi-9b",
        "gemini-2.5-flash",  // ← ADICIONAR
        "gemini-2.5-flash-lite",  // ← ADICIONAR
    ];
    // ...
}
```

**Vantagens**:
- ✅ Auto-detecta e usa agent loop
- ✅ Não precisa configurar manualmente
- ✅ Funciona para todos os usuários

**Desvantagens**:
- ⚠️ Requer mudança no código
- ⚠️ Pode não ser necessário se Gemini funcionar nativamente

### Solução 3: Usar Modelo com Function Calling Robusto

**Alternativas**:
```bash
# Usar Qwen (tem function calling robusto)
dev-jarvis.bat qwen

# Ou usar GPT-4o-mini via OpenRouter
dev-jarvis.bat free  # Pode usar modelos com function calling melhor
```

**Vantagens**:
- ✅ Function calling nativo funciona bem
- ✅ Mais rápido (sem loop client-side)
- ✅ Menos tokens

**Desvantagens**:
- ⚠️ Pode ser pago (dependendo do modelo)
- ⚠️ Não resolve o problema do Gemini

---

## Teste Recomendado

### Teste 1: Verificar se Gemini Gera Tool Calls

1. **Ativar logs detalhados**
2. **Pedir ao agente**: "Liste os arquivos em jarvis-rs/core/src"
3. **Verificar logs**:
   - Tool calls foram geradas?
   - Formato correto?
   - Resposta do modelo contém `tool_calls`?

### Teste 2: Forçar Agent Loop

1. **Configurar**:
   ```toml
   [agent_loop]
   mode = "text_based"
   ```
2. **Testar novamente**: "Liste os arquivos em jarvis-rs/core/src"
3. **Verificar**: Agora usa tools?

### Teste 3: Comparar com Outro Modelo

1. **Usar Qwen**: `dev-jarvis.bat qwen`
2. **Testar**: "Liste os arquivos em jarvis-rs/core/src"
3. **Comparar**: Qwen usa tools? Gemini não?

---

## Configuração Recomendada para Gemini

### Configuração Completa

```toml
model = "gemini-2.5-flash"

[agent_loop]
# Forçar text-based para garantir que tools funcionem
mode = "text_based"
base_url = "https://generativelanguage.googleapis.com/v1beta/openai"
api_key = "env:GOOGLE_API_KEY"
model = "gemini-2.5-flash"  # Opcional, usa model principal se não especificado
temperature = 0.7
max_tokens = 4096
max_iterations = 25
timeout_sec = 300
```

### Configuração Mínima (Se Auto-Detectar)

```toml
model = "gemini-2.5-flash"

[agent_loop]
mode = "auto"  # Tenta native primeiro, fallback para text_based se necessário
```

**Nota**: Se `mode = "auto"` não funcionar, forçar `text_based`.

---

## Conclusão

### Problema Identificado

**Causa provável**: `gemini-2.5-flash` pode não estar gerando tool calls corretamente via Responses API, mesmo sendo detectado como Native.

### Solução Imediata

**Forçar agent loop (text-based)** para Gemini:

```toml
[agent_loop]
mode = "text_based"
base_url = "https://generativelanguage.googleapis.com/v1beta/openai"
api_key = "env:GOOGLE_API_KEY"
```

### Solução de Longo Prazo

**Opção 1**: Adicionar Gemini à lista de modelos text-based no código  
**Opção 2**: Investigar por que Gemini não gera tool calls nativos  
**Opção 3**: Usar modelo alternativo com function calling mais robusto

---

## Referências

- `jarvis-rs/core/src/tools/text_tool_calling.rs:57` - Lista de modelos text-based
- `jarvis-rs/core/src/model_provider_info.rs:515` - Configuração Google provider
- `docs/reports/troubleshooting-agent-tools.md` - Troubleshooting geral
- `docs/reports/agent-loop-config-analysis.md` - Análise de configuração

---

**Última atualização**: 2026-02-18
