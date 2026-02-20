# Configuração do Jarvis CLI (Rust)

## Diferenças entre o projeto .NET e o CLI Rust

O projeto **Jarvis CLI (Rust)** usa um formato de configuração diferente do projeto **Jarvis AI (.NET)**:

### Projeto .NET (`appsettings.json`)
- Formato: **JSON**
- Localização: `src/Jarvis.CLI/appsettings.json`
- Estrutura: Hierárquica com seções como `LLM`, `Embeddings`, `RAG`, etc.

### CLI Rust (`config.toml`)
- Formato: **TOML**
- Localização: `~/.jarvis/config.toml` (Windows: `C:\Users\<usuario>\.jarvis\config.toml`)
- Estrutura: Baseada em `[model_providers.*]` e `[profiles.*]`

## Estrutura da Configuração

### 1. Provedores de Modelos (`[model_providers.*]`)

Cada provedor é definido em uma seção `[model_providers.<nome>]`:

```toml
[model_providers.databricks]
name = "Databricks"
base_url = "https://adb-926216925051160.0.azuredatabricks.net/serving-endpoints/{endpoint}/invocations"
env_key = "DATABRICKS_API_KEY"
wire_api = "responses"
http_headers = { "Content-Type" = "application/json" }
request_max_retries = 4
stream_max_retries = 10
stream_idle_timeout_ms = 300000
```

**Campos importantes:**
- `name`: Nome descritivo do provedor
- `base_url`: URL base do endpoint da API
- `env_key`: Nome da variável de ambiente que contém a chave da API
- `wire_api`: Protocolo usado (geralmente `"responses"`)
- `http_headers`: Headers HTTP adicionais (opcional)
- `query_params`: Parâmetros de query (opcional, usado principalmente para Azure)

### 2. Perfis (`[profiles.*]`)

Perfis permitem alternar entre diferentes configurações:

```toml
[profiles.default]
model_provider = "databricks"
model = "databricks-claude-opus-4-5"
```

**Campos:**
- `model_provider`: ID do provedor definido em `[model_providers.*]`
- `model`: Nome do modelo a ser usado

### 3. Configuração Global

```toml
# Provedor padrão
model_provider = "databricks"

# Perfil padrão
profile = "default"
```

## Mapeamento do appsettings.json para config.toml

### LLM Providers

| appsettings.json | config.toml |
|-----------------|------------|
| `LLM.Provider` | `model_provider` (global) |
| `LLM.Databricks.BaseUrl` | `[model_providers.databricks].base_url` |
| `LLM.Databricks.Token` | Variável de ambiente `DATABRICKS_API_KEY` |
| `LLM.Databricks.Models.*` | `[profiles.*].model` |

### Exemplo de Migração

**appsettings.json:**
```json
{
  "LLM": {
    "Provider": "databricks",
    "Databricks": {
      "BaseUrl": "https://adb-926216925051160.0.azuredatabricks.net",
      "Token": "your_databricks_api_key_here",
      "Models": {
        "Planner": "databricks-claude-opus-4-5",
        "Developer": "databricks-meta-llama-3-1-405b"
      }
    }
  }
}
```

**config.toml equivalente:**
```toml
# Provedor padrão
model_provider = "databricks"

# Configuração do provedor Databricks
[model_providers.databricks]
name = "Databricks"
base_url = "https://adb-926216925051160.0.azuredatabricks.net/serving-endpoints/{endpoint}/invocations"
env_key = "DATABRICKS_API_KEY"
wire_api = "responses"
http_headers = { "Content-Type" = "application/json" }

# Perfis
[profiles.planner]
model_provider = "databricks"
model = "databricks-claude-opus-4-5"

[profiles.developer]
model_provider = "databricks"
model = "databricks-meta-llama-3-1-405b"
```

**Variável de ambiente:**
```powershell
[System.Environment]::SetEnvironmentVariable("DATABRICKS_API_KEY", "your_databricks_api_key_here", "User")
```

## Configuração de Variáveis de Ambiente

O Jarvis CLI Rust usa variáveis de ambiente para armazenar chaves de API por segurança. Configure-as antes de usar:

### Windows (PowerShell)
```powershell
# Databricks
[System.Environment]::SetEnvironmentVariable("DATABRICKS_API_KEY", "sua-chave-aqui", "User")

# OpenAI
[System.Environment]::SetEnvironmentVariable("OPENAI_API_KEY", "sua-chave-aqui", "User")

# OpenRouter
[System.Environment]::SetEnvironmentVariable("OPENROUTER_API_KEY", "sua-chave-aqui", "User")

# Google AI Studio (Gemini) — free tier disponível em https://aistudio.google.com/apikey
[System.Environment]::SetEnvironmentVariable("GOOGLE_API_KEY", "sua-chave-aqui", "User")

# Azure OpenAI
[System.Environment]::SetEnvironmentVariable("AZURE_OPENAI_API_KEY", "sua-chave-aqui", "User")

# Anthropic
[System.Environment]::SetEnvironmentVariable("ANTHROPIC_API_KEY", "sua-chave-aqui", "User")
```

**Importante:** Após definir variáveis de ambiente, você precisa:
1. Fechar e reabrir o terminal/PowerShell
2. Ou reiniciar o sistema para que as variáveis sejam carregadas

## Testando a Configuração

1. **Verificar se o arquivo existe:**
   ```powershell
   Test-Path "$env:USERPROFILE\.jarvis\config.toml"
   ```

2. **Verificar variáveis de ambiente:**
   ```powershell
   $env:DATABRICKS_API_KEY
   ```

3. **Testar o Jarvis CLI:**
   ```bash
   jarvis chat
   ```

4. **Verificar logs (se houver problemas):**
   ```bash
   jarvis chat --verbose
   ```

## Daemon — Configuração de Pipeline com Google Gemini

O daemon suporta múltiplos provedores LLM para geração de conteúdo. Para usar o Google Gemini (free tier):

### 1. Obtenha a API Key

Acesse [Google AI Studio](https://aistudio.google.com/apikey) e crie uma chave gratuita.

### 2. Configure a variável de ambiente

Adicione ao arquivo `jarvis-rs/.env`:
```env
GOOGLE_API_KEY=AIzaSy...sua-chave-aqui
```

Ou defina como variável de ambiente (ver seção acima).

### 3. Use no pipeline config

```json
{
  "llm": {
    "provider": "google",
    "model": "gemini-2.0-flash"
  },
  "seo": {
    "niche": "Seu Nicho",
    "language": "pt-BR"
  }
}
```

Veja o exemplo completo em `jarvis-rs/daemon/examples/pipeline-google-gemini.json`.

### Provedores suportados pelo daemon

| Provider | Env Var | Modelo padrão | Custo |
|----------|---------|---------------|-------|
| `google` | `GOOGLE_API_KEY` | `gemini-2.0-flash` | Free tier |
| `openrouter` | `OPENROUTER_API_KEY` | `mistralai/mistral-nemo` | ~$0.03/M tokens |
| `openai` | `OPENAI_API_KEY` | `gpt-4o-mini` | ~$2/M tokens |
| `ollama` | — | `llama3.2` | Grátis (local) |
| `databricks` | `DATABRICKS_API_KEY` | `databricks-claude-haiku-4-5` | Variável |

## Notas Importantes

1. **Databricks Endpoint:** O `base_url` do Databricks deve incluir o endpoint específico no formato:
   ```
   https://{workspace}.cloud.databricks.com/serving-endpoints/{endpoint}/invocations
   ```
   Você precisará substituir `{endpoint}` pelo nome real do seu endpoint.

2. **Segurança:** Nunca commite o arquivo `config.toml` com chaves de API. Use variáveis de ambiente através do campo `env_key`.

3. **Múltiplos Perfis:** Você pode criar vários perfis e alternar entre eles usando:
   ```bash
   jarvis chat --profile planner
   jarvis chat --profile developer
   ```

4. **Validação:** O Jarvis CLI valida automaticamente a sintaxe TOML ao iniciar. Se houver erros, eles serão exibidos.

## Arquivos Criados

- `config.toml.example`: Exemplo de configuração completo
- `test-config.ps1`: Script PowerShell para testar a configuração
- `C:\Users\<usuario>\.jarvis\config.toml`: Arquivo de configuração do usuário

## Referências

- Documentação do Jarvis CLI: `jarvis-cli/docs/`
- Schema de configuração: `jarvis-cli/jarvis-rs/core/config.schema.json`
- Exemplos de código: `jarvis-cli/jarvis-rs/core/src/config/mod.rs`
