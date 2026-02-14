# AgentLoop Bridge — Text-Based Tool Calling Integration

**Data**: 2026-02-13
**Status**: Implementado
**Modulo**: `jarvis-rs/core/src/agent_loop/bridge.rs`

## Overview

Bridge que conecta o `AgentLoop` (Think-Execute-Observe cycle) ao sistema de tools existente, habilitando modelos sem suporte nativo a function calling (como Mistral-Nemo, Phi-3, TinyLlama, etc.) a utilizar tools via text-based tool calling.

## Arquitetura

```
                 ┌───────────────────┐
                 │    AgentLoop      │
                 │ (Think→Exec→Obs)  │
                 └────────┬──────────┘
                          │
              ┌───────────┴───────────┐
              │                       │
   ┌──────────┴──────────┐ ┌─────────┴──────────┐
   │  BridgeLlmClient    │ │ BridgeToolExecutor  │
   │  (AgentLlmClient)   │ │ (AgentToolExecutor) │
   └──────────┬──────────┘ └─────────┬──────────┘
              │                       │
   ┌──────────┴──────────┐ ┌─────────┴──────────┐
   │  OpenAI-compat API  │ │ Local tool dispatch │
   │  + text tool inject │ │ (shell, read, etc.) │
   └─────────────────────┘ └────────────────────┘
```

## Componentes

### BridgeLlmClient (`AgentLlmClient`)
- Conecta com qualquer API OpenAI-compativel (Ollama, OpenRouter, etc.)
- Injeta descricoes de tools no system prompt automaticamente
- Parseia tool calls do texto de resposta (formato `tool_call` fence)
- Suporta deteccao de tool calls inline (JSON sem fences)

### BridgeToolExecutor (`AgentToolExecutor`)
- Executa tools localmente sem necessidade do protocolo Responses API
- Tools suportados:
  - `shell` — execucao de comandos shell
  - `read_file` — leitura de arquivos com offset/limit
  - `list_directory` — listagem de diretorios
  - `grep_search` — busca via ripgrep
  - `write_new_file` — criacao de arquivos

### BridgeLlmConfig
```rust
BridgeLlmConfig {
    base_url: "http://localhost:11434/v1",  // Ollama default
    api_key: "",                             // Vazio para modelos locais
    model: "mistral",
    temperature: 0.7,
    max_tokens: 4096,
    timeout_sec: 120,
}
```

### BridgeToolConfig
```rust
BridgeToolConfig {
    working_dir: PathBuf::from("."),
    shell_timeout: Duration::from_secs(30),
    require_approval: false,
}
```

## Fluxo de Execucao

1. **System Prompt Enrichment**: O `BridgeLlmClient` injeta descricoes de tools no final do system prompt
2. **Think**: Envia mensagens para o LLM via API OpenAI-compatible
3. **Parse**: Extrai tool calls do texto de resposta (fenced ou inline)
4. **Execute**: `BridgeToolExecutor` despacha para o tool handler apropriado
5. **Observe**: Resultado formatado como `[Tool Result: ...]` e adicionado ao contexto
6. **Repeat**: Loop continua ate resposta final ou limite de iteracoes

## Modelos Detectados como Text-Based

O modulo `text_tool_calling.rs` detecta automaticamente modelos sem function calling nativo:
- `mistral-nemo`
- `phi-3`, `phi-2`
- `tinyllama`
- `stablelm`
- `gemma-2b`, `gemma-7b`
- `yi-6b`, `yi-9b`

## Testes

### Unitarios (bridge.rs)
- `bridge_llm_config_defaults` — Valores padrao da configuracao LLM
- `bridge_llm_config_deserialize` — Desserializacao JSON da config
- `bridge_tool_config_defaults` — Valores padrao da config de tools
- `bridge_tool_executor_schemas` — Geracao correta de schemas JSON
- `build_openai_messages_injects_tools` — Injecao de tools no system prompt
- `build_openai_messages_formats_tool_results` — Formatacao de resultados como user messages
- `exec_read_file_works` — Leitura de arquivo real
- `exec_list_directory_works` — Listagem de diretorio real
- `exec_unknown_tool_returns_error` — Erro para tool desconhecido
- `exec_write_and_read_roundtrip` — Escrita e leitura de arquivo
- `full_agent_loop_with_bridge` — Loop completo sem tools (mock LLM)
- `full_agent_loop_with_tool_call` — Loop completo com tool call (mock LLM + real executor)

### Unitarios (text_tool_calling.rs — pre-existentes)
- `detect_native_for_gpt4` — Deteccao de modo nativo
- `detect_text_based_for_mistral_nemo` — Deteccao de modo text-based
- `parse_fenced_tool_call` — Parse de tool call em fence
- `parse_multiple_fenced_tool_calls` — Parse de multiplos tool calls
- `parse_inline_json_tool_call` — Parse de tool call inline
- `prompt_injection_generates_valid_text` — Geracao de prompt injection
- `format_tool_result` — Formatacao de resultado de tool

## Arquivos

- `core/src/agent_loop/mod.rs` — Registro do modulo `bridge`
- `core/src/agent_loop/bridge.rs` — Implementacao do BridgeLlmClient e BridgeToolExecutor
- `core/src/tools/text_tool_calling.rs` — Text parsing e mode detection (pre-existente)

## Uso Futuro no TUI

O TUI pode utilizar o bridge de duas formas:

1. **Automatico**: Detectar o modelo e, se `ToolCallingMode::TextBased`, usar `AgentLoop` com o bridge em vez do protocolo Responses API
2. **Configuravel**: Permitir ao usuario selecionar o modo via config (`tool_calling_mode: "text_based"`)

A integracao no chatwidget requer routing do `AgentEvent` para `AppEvent`, que pode ser feito via um adaptador simples.
