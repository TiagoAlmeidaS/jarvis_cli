# Plano de Integração do Ollama com Jarvis CLI

## 📋 Resumo Executivo

**Objetivo**: Adicionar suporte para Ollama local usando a API Chat Completions (`/v1/chat/completions`)

**Status Atual**: ✅ **VIÁVEL** - Infraestrutura já existe!

**Estimativa**: Implementação Simples (2-4 horas)

---

## 🔍 Análise do Código Atual

### Descoberta Importante: Sistema de Conversão Já Existe!

O Jarvis **já possui** um sistema completo de conversão de Responses API → Chat Completions API implementado para o **Databricks**!

#### Localização da Infraestrutura

**Arquivo**: `jarvis-rs/jarvis-api/src/requests/responses.rs`

**Linhas 150-285**: Função `convert_to_chat_format()` que:
- ✅ Converte `instructions` + `input` → array de `messages`
- ✅ Transforma `ResponseItem` → formato de mensagens do Chat Completions
- ✅ Suporta system messages, user messages, assistant messages
- ✅ Converte function calls para tool_calls
- ✅ Trata function outputs como mensagens de usuário

**Linhas 162-168**: Sistema de path dinâmico:
```rust
let path = if provider.name.to_lowercase() == "databricks" {
    format!("serving-endpoints/{}/invocations", model)
} else {
    "responses".to_string()
};
```

---

## 🏗️ Arquitetura Atual

### 1. Definição de Providers

**Arquivo**: `jarvis-rs/core/src/model_provider_info.rs`

```rust
pub enum WireApi {
    #[default]
    Responses,  // Única opção atualmente
}
```

**Problema**: API Chat foi REMOVIDA (linha 32-34)
```rust
const CHAT_WIRE_API_REMOVED_ERROR: &str =
    "`wire_api = \"chat\"` is no longer supported.";
```

### 2. Cliente Ollama Existente

**Arquivo**: `jarvis-rs/ollama/src/client.rs`

- Já tem cliente HTTP para Ollama
- Usa endpoints nativos (`/api/tags`, `/api/pull`, `/api/version`)
- Detecta se está usando API OpenAI-compatible (linha 62)
- Probe server para verificar conectividade

### 3. Sistema de API

**Arquivo**: `jarvis-rs/jarvis-api/src/endpoint/responses.rs`

- `ResponsesClient` faz chamadas via `EndpointSession`
- Suporta streaming SSE
- Usa `ResponsesRequestBuilder` para construir requests

---

## 🎯 Estratégia de Implementação

### Abordagem: Reutilizar Sistema Databricks

Em vez de reintroduzir `wire_api = "chat"`, vamos usar a **mesma estratégia do Databricks**:
- Detectar provider pelo nome
- Aplicar conversão de formato
- Ajustar path dinamicamente

### Mudanças Necessárias

#### 1. Adicionar Flag de Chat Completions ao Provider

**Arquivo**: `jarvis-rs/core/src/model_provider_info.rs`

```rust
pub struct ModelProviderInfo {
    // ... campos existentes ...

    /// Se true, usa Chat Completions API em vez de Responses API
    #[serde(default)]
    pub uses_chat_completions_api: bool,
}
```

**Vantagem**: Não reintroduz `wire_api = "chat"` que foi removido

#### 2. Modificar Construção de Request

**Arquivo**: `jarvis-rs/jarvis-api/src/requests/responses.rs`

**Mudança na linha 150-153**:
```rust
// ANTES (apenas Databricks)
if provider.name.to_lowercase() == "databricks" {
    body = convert_to_chat_format(&body, model, instructions, input)?;
}

// DEPOIS (Databricks + Ollama + qualquer outro)
if should_use_chat_completions(&provider) {
    body = convert_to_chat_format(&body, model, instructions, input)?;
}
```

**Mudança na linha 162-168**:
```rust
// ANTES
let path = if provider.name.to_lowercase() == "databricks" {
    format!("serving-endpoints/{}/invocations", model)
} else {
    "responses".to_string()
};

// DEPOIS
let path = if provider.name.to_lowercase() == "databricks" {
    format!("serving-endpoints/{}/invocations", model)
} else if should_use_chat_completions(&provider) {
    "chat/completions".to_string()
} else {
    "responses".to_string()
};
```

**Função auxiliar**:
```rust
fn should_use_chat_completions(provider: &Provider) -> bool {
    provider.name.to_lowercase() == "databricks" ||
    provider.uses_chat_completions_api
}
```

#### 3. Configurar Ollama no `config.toml`

```toml
[model_providers.ollama]
name = "Ollama"
base_url = "http://localhost:11434/v1"
uses_chat_completions_api = true  # 🆕 Nova flag
```

#### 4. Atualizar Built-in Providers

**Arquivo**: `jarvis-rs/core/src/model_provider_info.rs` (linha 302-320)

```rust
pub fn create_oss_provider(default_provider_port: u16, wire_api: WireApi) -> ModelProviderInfo {
    let base_url = /* ... */;
    ModelProviderInfo {
        name: "gpt-oss".into(),
        base_url: Some(base_url),
        // ... outros campos ...
        uses_chat_completions_api: false,  // 🆕 Default
        // ...
    }
}
```

**Criar função específica para Ollama**:
```rust
pub fn create_ollama_provider() -> ModelProviderInfo {
    let base_url = std::env::var("OLLAMA_BASE_URL")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| format!("http://localhost:{}/v1", DEFAULT_OLLAMA_PORT));

    ModelProviderInfo {
        name: "Ollama".into(),
        base_url: Some(base_url),
        env_key: None,
        env_key_instructions: None,
        experimental_bearer_token: None,
        wire_api: WireApi::Responses,  // Mantém Responses (conversão interna)
        uses_chat_completions_api: true,  // 🆕 Flag que ativa conversão
        query_params: None,
        http_headers: None,
        env_http_headers: None,
        request_max_retries: None,
        stream_max_retries: None,
        stream_idle_timeout_ms: None,
        requires_openai_auth: false,
        supports_websockets: false,
    }
}
```

**Atualizar `built_in_model_providers()`** (linha 280-300):
```rust
pub fn built_in_model_providers() -> HashMap<String, ModelProviderInfo> {
    [
        ("databricks", create_databricks_provider(None)),
        ("openai", P::create_openai_provider()),
        ("ollama", create_ollama_provider()),  // 🆕 Usa função específica
        ("lmstudio", create_oss_provider(DEFAULT_LMSTUDIO_PORT, WireApi::Responses)),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_string(), v))
    .collect()
}
```

---

## 🔄 Fluxo de Dados Proposto

```
┌──────────────────┐
│   Jarvis CLI     │
│  (user input)    │
└────────┬─────────┘
         │
         ▼
┌────────────────────────────────┐
│  ModelClient                   │
│  - Cria ResponsesRequest       │
│  - Formato: Responses API      │
└────────┬───────────────────────┘
         │
         ▼
┌────────────────────────────────┐
│  ResponsesRequestBuilder       │
│  - Detecta provider            │
│  - if uses_chat_completions:   │
│    • convert_to_chat_format()  │
│    • path = "chat/completions" │
└────────┬───────────────────────┘
         │
         ▼
┌────────────────────────────────┐
│  HTTP POST                     │
│  → http://localhost:11434/v1/  │
│     chat/completions           │
│                                │
│  Body: {                       │
│    "model": "llama3.2:3b",     │
│    "messages": [...],          │
│    "stream": true              │
│  }                             │
└────────┬───────────────────────┘
         │
         ▼
┌────────────────────────────────┐
│  Ollama Server                 │
│  - Processa request            │
│  - Retorna SSE stream          │
└────────┬───────────────────────┘
         │
         ▼
┌────────────────────────────────┐
│  ResponsesClient               │
│  - Parse SSE events            │
│  - Converte para ResponseEvent │
└────────┬───────────────────────┘
         │
         ▼
┌────────────────────────────────┐
│  Jarvis CLI                    │
│  - Display resposta            │
└────────────────────────────────┘
```

---

## ✅ Checklist de Implementação

### Fase 1: Core Changes
- [ ] 1.1 Adicionar campo `uses_chat_completions_api: bool` em `ModelProviderInfo`
- [ ] 1.2 Atualizar schema JSON do config (`config.schema.json`)
- [ ] 1.3 Criar função `create_ollama_provider()`
- [ ] 1.4 Atualizar `built_in_model_providers()` para usar nova função

### Fase 2: API Request Changes
- [ ] 2.1 Criar função `should_use_chat_completions(provider: &Provider) -> bool`
- [ ] 2.2 Modificar condição de conversão (linha ~151)
- [ ] 2.3 Modificar construção de path (linha ~162)
- [ ] 2.4 Adicionar suporte ao Provider no `jarvis-api`

### Fase 3: Testes
- [ ] 3.1 Testar com Ollama local
- [ ] 3.2 Verificar compatibilidade com Databricks (não quebrar)
- [ ] 3.3 Verificar compatibilidade com OpenAI/OpenRouter
- [ ] 3.4 Testar streaming SSE

### Fase 4: Documentação
- [ ] 4.1 Atualizar `config.toml.example` com exemplo Ollama
- [ ] 4.2 Criar guia de setup Ollama
- [ ] 4.3 Documentar flag `uses_chat_completions_api`

---

## 🧪 Comandos de Teste

### Teste Básico
```bash
cd jarvis-rs
cargo build --release

# Teste com Ollama
./target/release/jarvis.exe exec \
  -c 'model_provider="ollama"' \
  -c 'model="llama3.2:3b"' \
  "Olá! Você está funcionando?"
```

### Teste Comparativo
```bash
# OpenRouter (Responses API)
./target/release/jarvis.exe exec \
  -c 'model_provider="openrouter"' \
  "Teste OpenRouter"

# Databricks (Chat Completions via conversão)
./target/release/jarvis.exe exec \
  -c 'model_provider="databricks"' \
  "Teste Databricks"

# Ollama (Chat Completions via conversão - NOVO)
./target/release/jarvis.exe exec \
  -c 'model_provider="ollama"' \
  "Teste Ollama"
```

---

## 🎉 Vantagens da Abordagem

1. **✅ Reutiliza Código Existente**: Usa `convert_to_chat_format()` do Databricks
2. **✅ Não Quebra Nada**: Mantém compatibilidade com providers existentes
3. **✅ Simples**: Apenas uma flag booleana no provider
4. **✅ Extensível**: Qualquer provider pode usar `uses_chat_completions_api = true`
5. **✅ Mantém Arquitetura**: Não reintroduz `wire_api = "chat"` removido

---

## 🚀 Próximos Passos

**Imediatos**:
1. Você aprova essa abordagem?
2. Quer que eu comece a implementar?
3. Alguma modificação na estratégia?

**Depois da Implementação**:
- Adicionar suporte a outros providers Chat Completions (LM Studio, LocalAI, etc.)
- Melhorar conversão de tools/function calling
- Otimizar max_tokens e outros parâmetros

---

## 📝 Notas de Implementação

### Compatibilidade com SSE
O Ollama suporta Server-Sent Events (SSE) na API Chat Completions:
```
POST /v1/chat/completions
Content-Type: application/json

{"model": "llama3.2:3b", "messages": [...], "stream": true}
```

Resposta (SSE):
```
data: {"id":"chatcmpl-123","object":"chat.completion.chunk",...}
data: {"id":"chatcmpl-123","object":"chat.completion.chunk",...}
data: [DONE]
```

**Compatibilidade**: ResponsesClient já suporta SSE streaming! ✅

### Conversão de Ferramentas (Tools)
Atualmente o `convert_to_chat_format` **pula tools** (linha 275-277):
```rust
// Note: Databricks has strict tool format requirements
// For now, we skip tools to ensure basic chat works
```

**Para Ollama**: Precisamos adicionar suporte a tools no futuro
- Ollama suporta function calling desde v0.3.0
- Formato similar ao OpenAI Chat Completions

---

## 📚 Referências

- [Ollama API Documentation](https://github.com/ollama/ollama/blob/main/docs/api.md#generate-a-chat-completion)
- [OpenAI Chat Completions API](https://platform.openai.com/docs/api-reference/chat)
- Código existente: `jarvis-rs/jarvis-api/src/requests/responses.rs:150-285`

---

**Criado em**: 2026-02-11
**Autor**: Claude Sonnet 4.5 + Tiago
**Status**: ✅ Pronto para Implementação
