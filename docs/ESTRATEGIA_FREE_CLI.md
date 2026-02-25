# 🆓 Guia: Usando Estratégia Free no Jarvis CLI

Este guia mostra como configurar e usar uma estratégia "Free" no Jarvis CLI, utilizando modelos gratuitos do OpenRouter e Google AI Studio.

---

## 📋 Índice

1. [Configuração da Estratégia Free](#1-configuração-da-estratégia-free)
2. [Criando um Profile Free](#2-criando-um-profile-free)
3. [Usando no CLI](#3-usando-no-cli)
4. [Modelos Free Disponíveis](#4-modelos-free-disponíveis)

---

## 1. Configuração da Estratégia Free

### 1.1 Adicionar Estratégia no `config.toml`

Edite o arquivo `~/.jarvis/config.toml` e adicione a seção de estratégias LLM:

```toml
# ============================================================================
# ESTRATÉGIAS DE ROTEAMENTO LLM (LLM Routing Strategies)
# ============================================================================

# Estratégia Free - Usa apenas modelos gratuitos
[llm.strategies.free]
primary = "openrouter/deepseek/deepseek-r1:free"
fallbacks = [
    "openrouter/google/gemini-2.0-flash:free",
    "openrouter/google/gemma-3-27b-it:free",
    "openrouter/stepfun/step-3.5-flash:free"
]

# Estratégia Free com Google AI Studio (requer GOOGLE_API_KEY)
[llm.strategies.free_google]
primary = "google/gemini-2.5-flash"
fallbacks = [
    "openrouter/deepseek/deepseek-r1:free",
    "openrouter/google/gemini-2.0-flash:free"
]
```

### 1.2 Configurar Provider OpenRouter

Certifique-se de que o OpenRouter está configurado:

```toml
[model_providers.openrouter]
# A API key será lida da variável de ambiente OPENROUTER_API_KEY
# Ou você pode configurar diretamente aqui (não recomendado por segurança)
```

**Importante sobre formato de modelos:**
- **Estratégias LLM** (`llm.strategies.*`): Usam formato completo `openrouter/deepseek/deepseek-r1:free` (usado pelo daemon)
- **Profiles e CLI**: Quando `model_provider=openrouter` é especificado, use apenas `deepseek/deepseek-r1:free` (sem prefixo `openrouter/`)

**Importante**: Configure a variável de ambiente:
```bash
# Windows PowerShell
$env:OPENROUTER_API_KEY = "sk-or-v1-..."

# Linux/Mac
export OPENROUTER_API_KEY="sk-or-v1-..."
```

Obtenha sua chave em: https://openrouter.ai/keys

---

## 2. Criando um Profile Free

### 2.1 Profile para OpenRouter Free

Adicione um profile no `config.toml`:

```toml
[profiles.free]
model_provider = "openrouter"
model = "openrouter/deepseek/deepseek-r1:free"
```

### 2.2 Profile para Google AI Studio Free

```toml
[profiles.free_google]
model_provider = "google"
model = "google/gemini-2.5-flash"
```

**Nota**: Para Google AI Studio, você precisa:
- Obter uma API key em: https://ai.google.dev
- Configurar: `$env:GOOGLE_API_KEY = "sua-chave-aqui"`

---

## 3. Usando no CLI

### 3.1 Usando o Profile Free

```bash
# Usando profile free (OpenRouter)
jarvis chat -p free

# Usando profile free_google (Google AI Studio)
jarvis chat -p free_google
```

### 3.2 Usando Override de Configuração

```bash
# Especificar modelo free diretamente
# IMPORTANTE: Quando model_provider=openrouter, use apenas o nome do modelo sem prefixo
jarvis chat -c model_provider=openrouter -m "deepseek/deepseek-r1:free"

# Ou usando modelo do Google
jarvis chat -c model_provider=google -m "gemini-2.5-flash"
```

### 3.3 Modo Exec (Não-interativo)

```bash
# Com profile
jarvis exec -p free "Explique o que é Rust"

# Com override (sem prefixo openrouter/ quando model_provider já está especificado)
jarvis exec -c model_provider=openrouter -m "deepseek/deepseek-r1:free" "Explique o que é Rust"
```

---

## 4. Modelos Free Disponíveis

### 4.1 OpenRouter Free Models

| Modelo | Descrição | Uso Recomendado |
|--------|-----------|-----------------|
| `deepseek/deepseek-r1:free` | Melhor raciocínio, pensamento profundo | Planejamento, análise crítica |
| `google/gemini-2.0-flash:free` | Equilíbrio qualidade/velocidade | Desenvolvimento, chat rápido |
| `google/gemma-3-27b-it:free` | Modelo grande e capaz | Tarefas complexas |
| `stepfun/step-3.5-flash:free` | Mínima latência | Respostas rápidas |
| `nvidia/nemotron-nano-9b-v2:free` | Rápido, suficiente | Busca, exploração |
| `openrouter/free` | Auto-roteamento | Fallback automático |

**Nota Importante**: Quando usar `model_provider=openrouter`, especifique apenas o nome do modelo sem o prefixo `openrouter/`. Por exemplo: `-m "deepseek/deepseek-r1:free"` e não `-m "openrouter/deepseek/deepseek-r1:free"`.

### 4.2 Google AI Studio Free Models

| Modelo | Descrição | Uso Recomendado |
|--------|-----------|-----------------|
| `google/gemini-2.5-flash` | Mais capaz, com thinking | Planejamento, desenvolvimento |
| `google/gemini-2.5-flash-lite` | Rápido e funcional | Chat rápido, tarefas simples |

---

## 🚀 Exemplos Práticos

### Exemplo 1: Chat Interativo com Free

```bash
# Usando profile
jarvis chat -p free

# Ou diretamente (sem prefixo openrouter/ quando model_provider já está especificado)
jarvis chat -c model_provider=openrouter -m "deepseek/deepseek-r1:free"
```

### Exemplo 2: Execução Não-Interativa

```bash
# Com profile
jarvis exec -p free "Crie uma função em Rust que calcula fibonacci"

# Com override (sem prefixo openrouter/ quando model_provider já está especificado)
jarvis exec \
  -c model_provider=openrouter \
  -m "deepseek/deepseek-r1:free" \
  "Crie uma função em Rust que calcula fibonacci"
```

### Exemplo 3: Usando Google AI Studio

```bash
# Configurar API key primeiro
$env:GOOGLE_API_KEY = "sua-chave"

# Usar no CLI
jarvis chat -p free_google

# Ou diretamente (sem prefixo google/ quando model_provider já está especificado)
jarvis chat -c model_provider=google -m "gemini-2.5-flash"
```

---

## ⚙️ Configuração Completa de Exemplo

Aqui está um exemplo completo de seção para adicionar ao `~/.jarvis/config.toml`:

```toml
# ============================================================================
# ESTRATÉGIAS LLM FREE
# ============================================================================

[llm.strategies.free]
primary = "openrouter/deepseek/deepseek-r1:free"
fallbacks = [
    "openrouter/google/gemini-2.0-flash:free",
    "openrouter/google/gemma-3-27b-it:free",
    "openrouter/stepfun/step-3.5-flash:free"
]

[llm.strategies.free_google]
primary = "google/gemini-2.5-flash"
fallbacks = [
    "openrouter/deepseek/deepseek-r1:free",
    "openrouter/google/gemini-2.0-flash:free"
]

# ============================================================================
# PROFILES FREE
# ============================================================================

[profiles.free]
model_provider = "openrouter"
model = "deepseek/deepseek-r1:free"

[profiles.free_google]
model_provider = "google"
model = "gemini-2.5-flash"
```

---

## 🔍 Verificando Configuração

Para verificar se a configuração está correta:

```bash
# Ver configuração atual
jarvis --help

# Testar conexão com modelo free
jarvis exec -p free "Olá, você está funcionando?"
```

---

## 📝 Notas Importantes

1. **Limites de Rate**: Modelos free podem ter limites de rate. Se receber erro 429, aguarde alguns segundos.

2. **Qualidade**: Modelos free podem ter qualidade inferior aos modelos pagos, mas são suficientes para muitas tarefas.

3. **API Keys**: 
   - OpenRouter: https://openrouter.ai/keys
   - Google AI Studio: https://ai.google.dev

4. **Fallback Automático**: As estratégias configuradas com `fallbacks` tentarão automaticamente o próximo modelo se o primário falhar.

5. **Estratégias vs Profiles**: 
   - **Estratégias** (`llm.strategies.*`) são usadas principalmente por pipelines do daemon
   - **Profiles** (`profiles.*`) são usados pelo CLI para configuração rápida

---

## 🆘 Troubleshooting

### Erro: "No API key found"
- Verifique se a variável de ambiente está configurada
- Para OpenRouter: `$env:OPENROUTER_API_KEY`
- Para Google: `$env:GOOGLE_API_KEY`

### Erro: "Model not found"
- Verifique se o nome do modelo está correto
- Modelos free do OpenRouter devem terminar com `:free`

### Erro: "Rate limit exceeded"
- Aguarde alguns segundos e tente novamente
- Modelos free têm limites de uso

---

## 📚 Referências

- [Documentação OpenRouter](https://openrouter.ai/docs)
- [Documentação Google AI Studio](https://ai.google.dev/docs)
- [Guia de Configuração do Jarvis](../config.md)
- [Comandos do CLI](../COMANDOS_CORRETOS.md)
