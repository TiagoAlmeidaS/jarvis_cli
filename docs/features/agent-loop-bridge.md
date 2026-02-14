# AgentLoop Bridge — Text-Based Tool Calling Integration

**Data**: 2026-02-13
**Status**: Implementado (Bridge + TUI Integration)
**Modulos**:
- `jarvis-rs/core/src/agent_loop/bridge.rs`
- `jarvis-rs/core/src/config/types.rs` (AgentLoopSettings)
- `jarvis-rs/tui/src/chatwidget/agent_loop_runner.rs`

## Overview

Bridge que conecta o `AgentLoop` (Think-Execute-Observe cycle) ao sistema de tools existente, habilitando modelos sem suporte nativo a function calling (como Mistral-Nemo, Phi-3, TinyLlama, etc.) a utilizar tools via text-based tool calling.

O TUI detecta automaticamente o modo baseado no modelo ativo e roteia mensagens atraves do AgentLoop quando necessario.

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

   ┌──────────────────────────────────────────┐
   │  TUI ChatWidget                          │
   │  ┌─────────────────────────────────────┐ │
   │  │ submit_user_message()               │ │
   │  │  ├─ agent_loop_tx (TextBased mode)  │ │
   │  │  └─ jarvis_op_tx  (Native mode)     │ │
   │  └─────────────────────────────────────┘ │
   │  ┌─────────────────────────────────────┐ │
   │  │ agent_loop_runner                   │ │
   │  │  AgentEvent -> EventMsg translation │ │
   │  └─────────────────────────────────────┘ │
   └──────────────────────────────────────────┘
```

## Configuracao

### config.toml

#### Exemplo com OpenRouter (recomendado)

```toml
[agent_loop]
mode = "auto"
base_url = "https://openrouter.ai/api/v1"
model = "mistralai/mistral-nemo"
temperature = 0.7
max_tokens = 4096
max_iterations = 25
timeout_sec = 300
max_context_tokens = 32000
```

A `api_key` e resolvida automaticamente a partir da env var `OPENROUTER_API_KEY`
quando a `base_url` contem "openrouter.ai". Alternativamente, use:

```toml
# Resolucao explicita via env var
api_key_env = "OPENROUTER_API_KEY"

# Ou diretamente no valor com syntax env:
api_key = "env:OPENROUTER_API_KEY"

# Ou valor literal (nao recomendado para versionamento)
api_key = "sk-or-..."
```

#### Exemplo com Ollama local

```toml
[agent_loop]
mode = "auto"
base_url = "http://localhost:11434/v1"
model = "mistral"
```

### Resolucao de API Key

A resolucao segue esta ordem de prioridade:

1. **`api_key_env`** — nome da env var a ser lida (ex: `"OPENROUTER_API_KEY"`)
2. **`api_key = "env:VAR"`** — syntax `env:` no campo api_key
3. **`api_key = "sk-..."`** — valor literal direto
4. **Auto-detect** — baseado na `base_url`:
   - `openrouter.ai` → `OPENROUTER_API_KEY`
   - `api.openai.com` → `OPENAI_API_KEY`
   - `generativelanguage.googleapis.com` → `GOOGLE_API_KEY`
   - `anthropic.com` → `ANTHROPIC_API_KEY`

### dev-jarvis.bat

Atalho rapido para testar com OpenRouter:

```batch
dev-jarvis.bat agent                              # Mistral Nemo via OpenRouter
dev-jarvis.bat agent mistralai/codestral-mamba-latest  # Custom model
```

Requer `OPENROUTER_API_KEY` definida no `.env` ou no ambiente.

### Modos

| Modo | Comportamento |
|------|---------------|
| `auto` (default) | Detecta automaticamente baseado no modelo. Modelos conhecidos sem function calling usam text-based; outros usam native. |
| `text_based` | Sempre usa o AgentLoop com text-based tool calling. Util para modelos locais via Ollama. |
| `native` | Sempre usa o fluxo padrão Responses API. |
| `disabled` | Desabilita tool calling completamente. |

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

### AgentLoopRunner (TUI)
- Recebe mensagens do chatwidget via channel
- Cria BridgeLlmClient + BridgeToolExecutor por turno
- Traduz AgentEvent -> EventMsg para o TUI renderizar
- Mapeamento de eventos:
  - `Thinking` -> `TurnStarted` + `BackgroundEvent`
  - `ExecutingTool` -> `ExecCommandBegin`
  - `ToolResult` -> `ExecCommandEnd`
  - `FinalResponse` -> `AgentMessage` + `TurnComplete`
  - `Error` -> `Error`
  - `MaxIterationsReached` / `Timeout` -> `Warning`

### ChatWidget Integration
- `maybe_init_agent_loop()` — chamado em todos os construtores
- Detecta o modo efetivo baseado no modelo ativo
- Armazena `agent_loop_tx` e `agent_loop_cancel`
- `submit_user_message()` roteia mensagens para o canal correto

## Modelos Detectados como Text-Based

O modulo `text_tool_calling.rs` detecta automaticamente modelos sem function calling nativo:
- `mistral-nemo`
- `phi-3`, `phi-2`
- `tinyllama`
- `stablelm`
- `gemma-2b`, `gemma-7b`
- `yi-6b`, `yi-9b`

## Testes

### Config (types.rs)
- `agent_loop_mode_default_is_auto` — Default e Auto
- `agent_loop_mode_deserialize` — Desserializacao de todos os modos
- `agent_loop_config_toml_defaults` — Valores padrao da config TOML
- `agent_loop_config_toml_custom_values` — Valores customizados
- `effective_mode_auto_detects_native` — Auto detecta Native para GPT-4
- `effective_mode_auto_detects_text_based` — Auto detecta TextBased para Mistral-Nemo
- `effective_mode_explicit_override` — Override explicito funciona
- `agent_loop_settings_default` — Comparacao completa de defaults

### Bridge (bridge.rs)
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

### TUI Runner (agent_loop_runner.rs)
- `agent_loop_message_construction` — Construcao de mensagem
- `settings_defaults_are_sane` — Defaults sao validos
- `effective_mode_auto_detects_text_based` — Auto-deteccao no TUI
- `effective_mode_explicit_override` — Override no TUI
- `emit_agent_event_does_not_panic` — Todos os eventos sao tratados

## Arquivos

### Core
- `core/src/agent_loop/mod.rs` — Registro do modulo `bridge`
- `core/src/agent_loop/bridge.rs` — BridgeLlmClient, BridgeToolExecutor, re-exports
- `core/src/config/types.rs` — AgentLoopMode, AgentLoopConfigToml, AgentLoopSettings
- `core/src/config/mod.rs` — Campo `agent_loop` em Config e ConfigToml
- `core/src/tools/text_tool_calling.rs` — Text parsing e mode detection (pre-existente)

### TUI
- `tui/src/chatwidget/agent_loop_runner.rs` — Runner que traduz AgentEvent -> EventMsg
- `tui/src/chatwidget.rs` — Integracao: maybe_init_agent_loop(), routing em submit_user_message()
