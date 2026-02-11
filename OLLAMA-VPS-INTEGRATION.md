# 🚀 Integração Ollama VPS via Tailscale

## 📊 Situação Atual

### ✅ Infraestrutura Existente

```
Máquina Local          Tailscale VPN         VPS (100.98.213.86)
┌─────────────┐       (túnel seguro)     ┌──────────────────────┐
│ Jarvis CLI  │ ─────────────────────▶   │ ✅ Ollama:11434      │
│ Windows     │  IP: 100.98.213.86       │ ✅ Qdrant:6333       │
│             │                           │ ✅ PostgreSQL:5432   │
└─────────────┘                           │ ✅ Redis:6379        │
                                          └──────────────────────┘
```

**Modelos já instalados na VPS**:
- `phi3:mini` (256MB)
- `llama3.2:3b` (2GB)
- `llama3.1:8b` (4.7GB)
- `qwen2.5:7b`
- `deepseek-coder:6.7b`
- `codellama:7b`
- `gemma2:2b`
- `nomic-embed-text` (para RAG)

### ❌ Problema Atual

**Erro** (descrito em `WORKAROUND_SEM_RAG.md`):
```
ERROR: error sending request for url (http://100.98.213.86:11434/v1/responses)
```

**Causa**:
```
Jarvis ─[POST]→ http://100.98.213.86:11434/v1/responses
                                             └─ ❌ Endpoint não existe no Ollama

Ollama espera: /v1/chat/completions (Chat Completions API)
Jarvis usa:    /v1/responses (Responses API)
```

**Resultado**: Incompatibilidade de APIs → 404 Not Found

---

## 🎯 Solução Proposta

Implementar suporte a Chat Completions API conforme [`OLLAMA-INTEGRATION-PLAN.md`](./OLLAMA-INTEGRATION-PLAN.md)

### Abordagem

**Reutilizar conversão Databricks** que já existe:
- Databricks também usa Chat Completions API
- Sistema de conversão já implementado em `jarvis-api/src/requests/responses.rs`
- Apenas estender para detectar Ollama

### Mudança na Configuração

**Antes** (config.toml.vps):
```toml
[model_providers.ollama]
name = "Ollama VPS"
base_url = "http://100.98.213.86:11434/v1"
```

**Depois** (adicionar flag):
```toml
[model_providers.ollama]
name = "Ollama VPS"
base_url = "http://100.98.213.86:11434/v1"
uses_chat_completions_api = true  # 🆕 Flag que ativa conversão automática
```

---

## 🔄 Fluxo de Requisição (Após Implementação)

```
┌────────────────────────────────────────────────────────────────────┐
│ 1. Usuário: ./jarvis.exe chat                                      │
│    Prompt: "Analise este código"                                   │
└────────────┬───────────────────────────────────────────────────────┘
             │
             ▼
┌────────────────────────────────────────────────────────────────────┐
│ 2. Jarvis Core: Cria request interno (Responses API format)        │
│    {                                                                │
│      "model": "llama3.2:3b",                                        │
│      "instructions": "...",                                         │
│      "input": [...]                                                 │
│    }                                                                │
└────────────┬───────────────────────────────────────────────────────┘
             │
             ▼
┌────────────────────────────────────────────────────────────────────┐
│ 3. ResponsesRequestBuilder: Detecta provider                       │
│    - Provider name: "Ollama VPS"                                   │
│    - uses_chat_completions_api: true                               │
│    → Aplica conversão!                                             │
└────────────┬───────────────────────────────────────────────────────┘
             │
             ▼
┌────────────────────────────────────────────────────────────────────┐
│ 4. convert_to_chat_format(): Transforma request                    │
│    {                                                                │
│      "model": "llama3.2:3b",                                        │
│      "messages": [                                                  │
│        {"role": "system", "content": "..."},                        │
│        {"role": "user", "content": "..."}                           │
│      ],                                                             │
│      "stream": true                                                 │
│    }                                                                │
│    Path: "chat/completions" (não "responses")                       │
└────────────┬───────────────────────────────────────────────────────┘
             │
             ▼ HTTP POST via Tailscale
┌────────────────────────────────────────────────────────────────────┐
│ 5. Ollama VPS (100.98.213.86:11434)                                │
│    POST /v1/chat/completions                                        │
│    ✅ Endpoint correto!                                             │
│    → Processa request                                               │
│    → Retorna SSE stream                                             │
└────────────┬───────────────────────────────────────────────────────┘
             │
             ▼ SSE Events via Tailscale
┌────────────────────────────────────────────────────────────────────┐
│ 6. Jarvis ResponsesClient: Parse SSE                                │
│    data: {"choices":[{"delta":{"content":"Hello"}}]}                │
│    → Converte para ResponseEvent                                    │
│    → Display no terminal                                            │
└────────────────────────────────────────────────────────────────────┘
```

---

## 📋 Checklist de Implementação

### Fase 1: Código Core ✅
- [ ] Adicionar `uses_chat_completions_api: bool` em `ModelProviderInfo`
- [ ] Atualizar `config.schema.json`
- [ ] Modificar `create_ollama_provider()` para incluir flag
- [ ] Atualizar `built_in_model_providers()`

### Fase 2: API Request ✅
- [ ] Criar `should_use_chat_completions(provider) -> bool`
- [ ] Modificar condição em `responses.rs:~151`
- [ ] Modificar path em `responses.rs:~162`

### Fase 3: Configuração VPS 🎯
- [ ] Atualizar `config.toml.vps` com flag
- [ ] Copiar para `~/.jarvis/config.toml` na máquina local
- [ ] Verificar conexão Tailscale

### Fase 4: Testes na VPS 🧪
- [ ] Compilar: `cargo build --release`
- [ ] Teste conexão: `curl http://100.98.213.86:11434/api/tags`
- [ ] Teste Jarvis: `./jarvis.exe exec -c 'model_provider="ollama"' "teste"`
- [ ] Teste chat: `./jarvis.exe chat`
- [ ] Verificar logs: `journalctl -u ollama -f` (na VPS)

---

## 🧪 Comandos de Teste

### 1. Verificar Conectividade Tailscale

```bash
# Na máquina local (Git Bash)
ping 100.98.213.86

# Testar porta Ollama
curl http://100.98.213.86:11434/api/tags
# Deve retornar JSON com lista de modelos
```

### 2. Testar Ollama na VPS

```bash
# SSH na VPS
ssh root@100.98.213.86

# Ver modelos instalados
ollama list

# Ver status do serviço
systemctl status ollama

# Ver logs em tempo real
journalctl -u ollama -f
```

### 3. Testar Jarvis (Após Implementação)

```bash
cd /e/projects/ia/jarvis_cli/jarvis-rs

# Compilar
cargo build --release

# Teste 1: Exec simples
./target/release/jarvis.exe exec \
  -c 'model_provider="ollama"' \
  -c 'model="llama3.2:3b"' \
  "Olá! Você está rodando na VPS via Tailscale?"

# Teste 2: Chat interativo
./target/release/jarvis.exe chat \
  -c 'model_provider="ollama"' \
  -c 'model="llama3.2:3b"'

# Teste 3: Diferentes modelos
./target/release/jarvis.exe exec \
  -c 'model_provider="ollama"' \
  -c 'model="qwen2.5:7b"' \
  "Escreva uma função Rust que soma dois números"
```

### 4. Teste Comparativo

```bash
# Databricks (já funciona com conversão)
./target/release/jarvis.exe exec \
  -c 'model_provider="databricks"' \
  "teste databricks"

# Ollama VPS (após implementação)
./target/release/jarvis.exe exec \
  -c 'model_provider="ollama"' \
  "teste ollama"
```

---

## 🔍 Troubleshooting

### Problema: Não conecta à VPS

```bash
# 1. Verificar Tailscale na máquina local
tailscale status

# 2. Verificar Tailscale na VPS
ssh root@100.98.213.86
tailscale status

# 3. Testar conectividade
ping 100.98.213.86
curl http://100.98.213.86:11434/api/tags
```

### Problema: Ollama não responde

```bash
# Na VPS
ssh root@100.98.213.86

# Status do serviço
systemctl status ollama

# Reiniciar se necessário
systemctl restart ollama

# Ver logs
journalctl -u ollama -n 50 --no-pager
```

### Problema: Modelo não encontrado

```bash
# Na VPS
ollama list

# Se modelo não estiver instalado
ollama pull llama3.2:3b
```

### Problema: Jarvis ainda tenta /v1/responses

**Causa**: Implementação não concluída ou config incorreta

**Verificar**:
1. Flag `uses_chat_completions_api = true` está no config?
2. Código foi recompilado após mudanças?
3. Logs mostram qual endpoint está sendo usado?

```bash
# Ver logs detalhados
RUST_LOG=debug ./target/release/jarvis.exe exec \
  -c 'model_provider="ollama"' \
  "teste" 2>&1 | grep -i "endpoint\|url\|request"
```

---

## 📊 Configuração Completa VPS

### config.toml (em ~/.jarvis/)

```toml
# ============================================================================
# Ollama VPS via Tailscale
# ============================================================================
[model_providers.ollama]
name = "Ollama VPS"
base_url = "http://100.98.213.86:11434/v1"
uses_chat_completions_api = true  # 🆕 Ativa conversão para Chat Completions

# Modelos disponíveis na VPS
[model_providers.ollama.models]
planner = "qwen2.5:7b"
developer = "deepseek-coder:6.7b"
reviewer = "codellama:7b"
fast_chat = "gemma2:2b"
general = "llama3.2:3b"
advanced = "llama3.1:8b"
safety = "phi3:mini"
embeddings = "nomic-embed-text"

# ============================================================================
# RAG via VPS
# ============================================================================
[rag]
enabled = true
vector_store = "qdrant"
chunk_size = 512
chunk_overlap = 50
max_results = 10

[rag.qdrant]
url = "http://100.98.213.86:6333"
collection_name = "jarvis_knowledge"

[rag.embeddings]
provider = "ollama"
model = "nomic-embed-text"
base_url = "http://100.98.213.86:11434"

# ============================================================================
# Database (PostgreSQL na VPS)
# ============================================================================
[database]
provider = "postgres"
host = "100.98.213.86"
port = 5432
database = "jarvis"
username = "jarvis"
password = "jarvis_secure_password_2026"
ssl_mode = "disable"  # Tailscale já é VPN criptografada

# ============================================================================
# Cache (Redis na VPS)
# ============================================================================
[cache]
enabled = true
provider = "redis"
host = "100.98.213.86"
port = 6379
database = 0
ttl_seconds = 3600

# ============================================================================
# Performance (timeouts maiores via Tailscale)
# ============================================================================
[performance]
http_timeout_seconds = 60
embedding_timeout_seconds = 120
vector_search_timeout_seconds = 30
```

---

## 🎉 Resultado Esperado

Após implementação:

```bash
$ cd jarvis-rs
$ ./target/release/jarvis.exe chat

╭──────────────────────────────────────────────────╮
│ >_ Ollama Jarvis (v0.0.0)                        │
│ provider: Ollama VPS (via Tailscale)             │
│ model:  llama3.2:3b                              │
│ endpoint: http://100.98.213.86:11434             │
╰──────────────────────────────────────────────────╯

› Olá! Você está rodando na VPS?

✅ Sim! Estou rodando via Ollama na VPS (100.98.213.86)
   através do túnel seguro Tailscale.

   Modelos disponíveis:
   - llama3.2:3b (atual)
   - llama3.1:8b
   - qwen2.5:7b
   - deepseek-coder:6.7b
   - E mais! 🚀

› /model qwen2.5:7b

Modelo alterado para: qwen2.5:7b ✅

› Me ajude a escrever um código Rust

Claro! Vou ajudar... [resposta do modelo]
```

---

## 💡 Vantagens da Configuração VPS

| Vantagem | Descrição |
|----------|-----------|
| 🚀 **Performance** | Hardware dedicado com mais recursos |
| 💾 **Modelos Grandes** | 8GB+ RAM disponível na VPS |
| 🌐 **Sempre Disponível** | Acesso de qualquer dispositivo via Tailscale |
| 🔐 **Seguro** | Túnel VPN criptografado (não exposto publicamente) |
| 💰 **Custo Zero** | Além do custo da VPS que já possui |
| 🎯 **Multi-dispositivo** | Mesmo Ollama para PC, notebook, celular |

---

## 🚀 Próximos Passos

1. **✅ Revisar este documento**
2. **🔧 Implementar mudanças** conforme `OLLAMA-INTEGRATION-PLAN.md`
3. **🧪 Testar localmente** (se tiver Ollama local)
4. **🌐 Testar VPS via Tailscale**
5. **📝 Documentar resultados**

---

**Relacionado**:
- [`OLLAMA-INTEGRATION-PLAN.md`](./OLLAMA-INTEGRATION-PLAN.md) - Plano técnico detalhado
- [`OLLAMA_VPS_SETUP.md`](./OLLAMA_VPS_SETUP.md) - Setup inicial da VPS
- [`config.toml.vps`](./config.toml.vps) - Configuração completa VPS
- [`WORKAROUND_SEM_RAG.md`](./WORKAROUND_SEM_RAG.md) - Problema original

---

**Status**: 📋 Pronto para Implementação
**Data**: 2026-02-11
**Autor**: Claude Sonnet 4.5 + Tiago
