# 🎯 Modelos para Desenvolvimento - Guia Completo

## 🆓 Opção 1: Ollama (GRATUITO - RECOMENDADO)

### Por que Ollama?

✅ **Gratuito** - Sem custos
✅ **Local** - Roda na sua máquina
✅ **Sem limites** - Use quanto quiser
✅ **Privado** - Seus dados não saem da máquina
✅ **Rápido** - Resposta instantânea

### Setup Rápido

```bash
# 1. Instalar
winget install Ollama.Ollama

# 2. Baixar modelo (escolha um)
ollama pull llama3.2:3b      # Leve e rápido (2GB)
ollama pull phi3:mini         # Ultra-leve (256MB)
ollama pull llama3.1:8b       # Mais capaz (4.7GB)

# 3. Testar
cd jarvis-rs
./target/debug/jarvis.exe chat -c model_provider=ollama -m llama3.2:3b
```

### Modelos Recomendados

| Modelo | Tamanho | Uso | Velocidade |
|--------|---------|-----|------------|
| `phi3:mini` | 256MB | Testes rápidos | ⚡⚡⚡⚡⚡ |
| `llama3.2:3b` | 2GB | Desenvolvimento | ⚡⚡⚡⚡ |
| `llama3.1:8b` | 4.7GB | Uso geral | ⚡⚡⚡ |
| `codellama:7b` | 3.8GB | Programação | ⚡⚡⚡ |
| `llama3.1:70b` | 40GB | Produção | ⚡⚡ |

---

## 🆓 Opção 2: LM Studio (GRATUITO)

### Por que LM Studio?

✅ **Gratuito** - Sem custos
✅ **Interface gráfica** - Mais fácil
✅ **Local** - Privacidade

### Setup

```bash
# 1. Baixar de: https://lmstudio.ai/

# 2. Na interface, baixar modelo

# 3. Iniciar servidor local (porta 1234)

# 4. Testar
cd jarvis-rs
./target/debug/jarvis.exe chat -c model_provider=lmstudio
```

---

## 💰 Opção 3: OpenRouter (MUITO BARATO)

### Por que OpenRouter?

✅ **Muito barato** - $0.000001 por token para alguns modelos
✅ **Muitos modelos** - Acesso a vários providers
✅ **Sem setup** - Apenas API key

### Modelos Ultra-Baratos

| Modelo | Preço (1M tokens) | Uso |
|--------|-------------------|-----|
| `meta-llama/llama-3.2-3b-instruct` | $0.06 | Testes |
| `google/gemini-flash-1.5` | $0.075 | Rápido |
| `anthropic/claude-3-haiku` | $0.25 | Balanceado |

### Setup

```bash
# Já configurado no .env!
# Apenas use:
cd jarvis-rs
./target/debug/jarvis.exe chat -c model_provider=openrouter -m meta-llama/llama-3.2-3b-instruct
```

---

## 🚀 Opção 4: Groq (QUASE GRATUITO)

### Por que Groq?

✅ **Muito rápido** - Processamento super otimizado
✅ **Quota gratuita** - 14,400 requests/dia grátis
✅ **Sem setup local** - API na nuvem

### Adicionar Groq

Edite `~/.jarvis/config.toml`:

```toml
[model_providers.groq]
name = "Groq"
base_url = "https://api.groq.com/openai/v1"
env_key = "GROQ_API_KEY"
```

Depois:

```bash
export GROQ_API_KEY="sua_key_aqui"
./target/debug/jarvis.exe chat -c model_provider=groq -m llama-3.1-8b-instant
```

---

## 📊 Comparação de Custos

### Para 1 Milhão de Tokens de Teste

| Provider | Custo | Velocidade | Setup |
|----------|-------|------------|-------|
| **Ollama** | $0 | ⚡⚡⚡⚡ | Local |
| **LM Studio** | $0 | ⚡⚡⚡⚡ | Local |
| **Groq (free tier)** | $0 | ⚡⚡⚡⚡⚡ | Fácil |
| **OpenRouter (Llama)** | $0.06 | ⚡⚡⚡ | Fácil |
| **OpenAI (GPT-4o)** | $5.00 | ⚡⚡⚡ | Fácil |
| **Databricks** | ??? | ⚡⚡ | Complexo |

---

## 🎯 Estratégia Recomendada

### Fase 1: Desenvolvimento (Use Ollama)
```bash
# Testes rápidos e iteração
ollama pull llama3.2:3b
./target/debug/jarvis.exe chat -c model_provider=ollama -m llama3.2:3b
```

### Fase 2: Validação (Use OpenRouter)
```bash
# Testar com modelos melhores mas baratos
./target/debug/jarvis.exe chat -c model_provider=openrouter -m meta-llama/llama-3.2-3b-instruct
```

### Fase 3: Produção (Use OpenAI ou Databricks)
```bash
# Quando tudo estiver funcionando
./target/debug/jarvis.exe chat -c model_provider=openai -m gpt-4o
```

---

## 🔧 Configuração Multi-Provider

Edite `~/.jarvis/config.toml`:

```toml
# Provider padrão para desenvolvimento
model_provider = "ollama"
model = "llama3.2:3b"

[model_providers.ollama]
name = "Ollama"
base_url = "http://localhost:11434/v1"

# Para quando precisar de mais qualidade
[model_providers.openrouter]
name = "OpenRouter"
base_url = "https://openrouter.ai/api/v1"
env_key = "OPENROUTER_API_KEY"

# Para produção
[model_providers.openai]
name = "OpenAI"
base_url = "https://api.openai.com/v1"
```

---

## 💡 Dicas

### Economia Máxima
```bash
# Use phi3:mini para testes rápidos
ollama pull phi3:mini
./target/debug/jarvis.exe chat -c model_provider=ollama -m phi3:mini
```

### Qualidade com Economia
```bash
# Use llama3.1:8b para desenvolvimento real
ollama pull llama3.1:8b
./target/debug/jarvis.exe chat -c model_provider=ollama -m llama3.1:8b
```

### Produção
```bash
# Só use OpenAI quando estiver tudo pronto
./target/debug/jarvis.exe chat -c model_provider=openai -m gpt-4o
```

---

## 🚀 Quick Start

### Opção A: Script Automático

```bash
cd /e/projects/ia/jarvis_cli
chmod +x setup-ollama.sh
./setup-ollama.sh
```

### Opção B: Manual

```bash
# 1. Instalar Ollama
winget install Ollama.Ollama

# 2. Baixar modelo
ollama pull llama3.2:3b

# 3. Testar
cd jarvis-rs
./target/debug/jarvis.exe chat -c model_provider=ollama -m llama3.2:3b
```

---

## ✅ Resumo

| Fase | Provider | Modelo | Custo |
|------|----------|--------|-------|
| **Desenvolvimento** | Ollama | llama3.2:3b | $0 |
| **Testes** | OpenRouter | llama-3.2-3b | ~$0.06/1M |
| **Validação** | OpenRouter | claude-haiku | ~$0.25/1M |
| **Produção** | OpenAI | gpt-4o | ~$5/1M |

---

**Use Ollama para desenvolvimento = ZERO custos!** 🎉
