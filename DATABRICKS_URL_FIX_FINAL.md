# 🎯 Solução Definitiva: URLs Dinâmicas do Databricks

## ✅ Como Funciona CORRETAMENTE

O Jarvis CLI **JÁ TEM** um sistema inteligente que constrói URLs dinamicamente para Databricks!

### 📋 Código Automático (jarvis-api/src/requests/responses.rs:156-163)

```rust
// Build path dynamically for Databricks provider
let path = if provider.name.to_lowercase() == "databricks" {
    // For Databricks, construct path: serving-endpoints/{model}/invocations
    format!("serving-endpoints/{}/invocations", model)
} else {
    // For other providers (OpenAI, etc.), use standard "responses" path
    "responses".to_string()
};
```

### 🔧 Configuração Correta

**Base URL (SEM endpoint específico):**
```bash
DATABRICKS_BASE_URL=https://adb-926216925051160.0.azuredatabricks.net
```

**O sistema automaticamente adiciona o path dinâmico:**
```
{base_url}/serving-endpoints/{modelo}/invocations
```

### 🎭 URLs Construídas Automaticamente

#### Claude Opus 4.5
```
https://adb-926216925051160.0.azuredatabricks.net/serving-endpoints/databricks-claude-opus-4-5/invocations
```

#### Claude Haiku 4.5
```
https://adb-926216925051160.0.azuredatabricks.net/serving-endpoints/databricks-claude-haiku-4-5/invocations
```

#### Llama 3.1 405B
```
https://adb-926216925051160.0.azuredatabricks.net/serving-endpoints/databricks-meta-llama-3-1-405b/invocations
```

---

## 📝 Arquivos de Configuração

### 1. `.env` (E:\projects\ia\jarvis_cli\.env)

```bash
# Databricks Configuration
DATABRICKS_API_KEY=your_databricks_api_key_here
DATABRICKS_BASE_URL=https://your-workspace.azuredatabricks.net
```

### 2. `config.toml` (C:\Users\tiago\.jarvis\config.toml)

```toml
[model_providers.databricks]
name = "Databricks"
base_url = "https://adb-926216925051160.0.azuredatabricks.net"
env_key = "DATABRICKS_API_KEY"

[model_providers.databricks.models]
planner = "databricks-claude-opus-4-5"
developer = "databricks-meta-llama-3-1-405b"
reviewer = "databricks-meta-llama-3-1-405b"
fast_chat = "databricks-claude-haiku-4-5"
```

---

## 🚀 Como Testar

### 1. Carregar as variáveis de ambiente
```bash
# Linux/macOS/Git Bash
source ./configure-credentials.sh

# PowerShell
.\configure-credentials.ps1
```

### 2. Executar o Jarvis CLI
```bash
cd jarvis-rs
cargo build --release
./target/release/jarvis.exe chat
```

### 3. Ou rodar direto com Databricks
```bash
./target/release/jarvis.exe chat -c model_provider=databricks -m databricks-claude-opus-4-5
```

---

## ⚠️ Erros Comuns

### ❌ ERRO: URL com endpoint hardcoded
```bash
# ERRADO - Não funciona com múltiplos modelos
DATABRICKS_BASE_URL=https://adb-926216925051160.0.azuredatabricks.net/serving-endpoints/databricks-claude-opus-4-5/invocations
```

**Resultado:** URL duplicada
```
https://adb-926216925051160.0.azuredatabricks.net/serving-endpoints/databricks-claude-opus-4-5/invocations/serving-endpoints/databricks-claude-opus-4-5/invocations
```

### ✅ CORRETO: URL base apenas
```bash
# CORRETO - Funciona com todos os modelos
DATABRICKS_BASE_URL=https://adb-926216925051160.0.azuredatabricks.net
```

**Resultado:** URL correta construída dinamicamente
```
https://adb-926216925051160.0.azuredatabricks.net/serving-endpoints/{modelo_atual}/invocations
```

---

## 🎯 Benefícios da Solução

✅ **Múltiplos modelos:** Troca entre Opus, Haiku, Llama sem reconfigurar
✅ **Código limpo:** Não precisa hardcodar URLs
✅ **Agents funcionam:** Cada agent pode usar um modelo diferente
✅ **Manutenível:** Um único ponto de configuração
✅ **Flexível:** Funciona com qualquer modelo Databricks

---

## 📊 Fluxo de Construção da URL

```
┌─────────────────────────────────────────────────────────┐
│ 1. Configuração Base                                    │
│    DATABRICKS_BASE_URL=https://workspace.databricks.net │
└────────────────────┬────────────────────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────────────────────┐
│ 2. ModelProviderInfo                                     │
│    name: "Databricks"                                    │
│    base_url: from env or config                          │
└────────────────────┬────────────────────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────────────────────┐
│ 3. Request Builder (responses.rs:156-163)               │
│    if provider.name == "databricks":                     │
│      path = f"serving-endpoints/{model}/invocations"    │
└────────────────────┬────────────────────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────────────────────┐
│ 4. URL Final                                             │
│    {base_url}/{path}                                     │
│    = https://workspace/serving-endpoints/opus/invocations│
└─────────────────────────────────────────────────────────┘
```

---

## 💡 Resumo

| Aspecto | Valor |
|---------|-------|
| **Solução** | ✅ URLs dinâmicas já implementadas |
| **Config** | URL base SEM path específico |
| **Modelos** | Ilimitados, trocados dinamicamente |
| **Agents** | ✅ Funcionam com modelos diferentes |
| **Status** | ✅ PRONTO PARA USO |

---

## 🎉 Teste Agora!

```bash
# 1. Configure
source ./configure-credentials.sh

# 2. Execute
cd jarvis-rs
./target/release/jarvis.exe chat

# 3. Teste diferentes modelos
> /model databricks-claude-opus-4-5
> /model databricks-claude-haiku-4-5
> /model databricks-meta-llama-3-1-405b
```

**Todos os modelos funcionarão sem reconfiguração!** 🚀
