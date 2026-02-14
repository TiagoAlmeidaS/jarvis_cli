# ✅ Comandos Corretos do Jarvis CLI

## 🎯 Como Especificar o Provider

### ❌ ERRADO
```bash
# Isso NÃO FUNCIONA - flag --provider não existe!
./target/debug/jarvis.exe chat --provider databricks
```

### ✅ CORRETO

#### Opção 1: Via flag -c (Override de configuração)
```bash
./target/debug/jarvis.exe chat -c model_provider=databricks
```

#### Opção 2: Via profile -p
```bash
./target/debug/jarvis.exe chat -p databricks
```

#### Opção 3: Confiar no default (config.toml)
```bash
./target/debug/jarvis.exe chat
# Usa o provider definido em ~/.jarvis/config.toml
```

---

## 📋 Flags Disponíveis

### Chat Command
```bash
jarvis chat [OPTIONS]

Opções:
  -c, --config <key=value>           Override configuration
                                     Exemplo: -c model_provider=databricks
                                     Exemplo: -c 'model="o3"'

  -p, --profile <CONFIG_PROFILE>     Use profile específico

  -m, --model <MODEL>                Especificar modelo
                                     Exemplo: -m databricks-claude-opus-4-5

  --local-provider <OSS_PROVIDER>    Usar provider local
                                     Valores: lmstudio, ollama

  --oss                              Atalho para provider local

  -h, --help                         Mostrar ajuda
```

---

## 🚀 Executar com Databricks (Quick Start)

### Script Automático
```bash
cd /e/projects/ia/jarvis_cli
./scripts/RUN_JARVIS.sh
```

### Comando Manual
```bash
# 1. Configurar variáveis de ambiente
cd /e/projects/ia/jarvis_cli
source ./configure-credentials.sh

# 2. Executar
cd jarvis-rs
./target/debug/jarvis.exe chat -c model_provider=databricks
```

### Com modelo específico
```bash
./target/debug/jarvis.exe chat \
  -c model_provider=databricks \
  -m databricks-claude-opus-4-5
```

---

## 🔧 Override de Múltiplas Configurações

```bash
# Usar Databricks com modelo específico
./target/debug/jarvis.exe chat \
  -c model_provider=databricks \
  -c model=databricks-claude-opus-4-5

# Mudar permissões de sandbox
./target/debug/jarvis.exe chat \
  -c 'sandbox_permissions=["disk-full-read-access"]'

# Combinar múltiplos overrides
./target/debug/jarvis.exe chat \
  -c model_provider=databricks \
  -c model=databricks-meta-llama-3-1-405b \
  -c 'sandbox_permissions=["network-access"]'
```

---

## 📍 Providers Disponíveis

- `databricks` - Databricks serving endpoints (DEFAULT)
- `openai` - OpenAI API
- `openrouter` - OpenRouter API
- `ollama` - Ollama local
- `lmstudio` - LM Studio local

---

## 🆘 Troubleshooting

### Verificar qual provider está sendo usado
```bash
# Ver configuração atual
cat ~/.jarvis/config.toml | grep model_provider

# Executar com logs
RUST_LOG=debug ./target/debug/jarvis.exe chat
```

### Verificar credenciais
```bash
echo "DATABRICKS_API_KEY: ${DATABRICKS_API_KEY:0:10}..."
echo "DATABRICKS_BASE_URL: $DATABRICKS_BASE_URL"
```

### Forçar provider específico
```bash
# SEMPRE use -c ao invés de --provider
./target/debug/jarvis.exe chat -c model_provider=databricks
```

---

## 💡 Modelos Databricks Configurados

- **Planner**: `databricks-claude-opus-4-5`
- **Developer**: `databricks-meta-llama-3-1-405b`
- **Reviewer**: `databricks-meta-llama-3-1-405b`
- **FastChat**: `databricks-claude-haiku-4-5`

Para usar um modelo específico:
```bash
./target/debug/jarvis.exe chat \
  -c model_provider=databricks \
  -m databricks-claude-opus-4-5
```
