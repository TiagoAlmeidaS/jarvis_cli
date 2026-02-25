# 🗄️ Integração com Banco de Dados

## 📌 Resumo: **SIM, integra automaticamente!**

O `jarvis-web-api` **já integra** com o banco de dados através do `jarvis-core`. Não precisa configurar nada extra!

---

## 🔄 Como Funciona

### **Arquitetura de Integração**

```
┌─────────────────────────────────────┐
│      jarvis-web-api (Axum)          │
│                                      │
│  ┌──────────────────────────────┐  │
│  │   jarvis-core (Biblioteca)    │  │
│  │                                │  │
│  │  ┌────────────────────────┐  │  │
│  │  │  ThreadManager         │  │  │
│  │  │  RolloutRecorder       │  │  │
│  │  │  AuthManager           │  │  │
│  │  └────────────────────────┘  │  │
│  │           │                   │  │
│  │           ▼                   │  │
│  │  ┌────────────────────────┐  │  │
│  │  │  SQLite Database        │  │  │
│  │  │  (arquivos em disco)    │  │  │
│  │  └────────────────────────┘  │  │
│  └──────────────────────────────┘  │
│           │                          │
│           ▼                          │
│  ~/.jarvis/ (ou JARVIS_HOME)        │
│  ├── config.toml                     │
│  ├── rollout-*.jsonl (threads)        │
│  ├── credentials.json (opcional)      │
│  └── outros arquivos...              │
└──────────────────────────────────────┘
```

---

## 💾 Banco de Dados: **SQLite**

### **O que é usado?**

O Jarvis usa **SQLite** (banco de dados em arquivo), mas **não é um banco tradicional**.

Na verdade, o Jarvis armazena dados em:
- **Arquivos JSONL** (rollout files) para threads/conversas
- **Arquivos de configuração** (config.toml)
- **Arquivos de credenciais** (credentials.json ou keyring)

### **Localização dos Dados**

Todos os dados ficam em `jarvis_home` (padrão: `~/.jarvis`):

```bash
~/.jarvis/
├── config.toml              # Configuração principal
├── rollout-*.jsonl           # Threads/conversas (um arquivo por thread)
├── credentials.json          # Credenciais (se usar modo "file")
├── log/                      # Logs
└── outros arquivos...
```

### **Como o web-api acessa?**

```rust
// server.rs - Linha 74
let jarvis_home = find_jarvis_home()?;  // Encontra ~/.jarvis ou JARVIS_HOME

// server.rs - Linha 77-81
let config = ConfigBuilder::default()
    .jarvis_home(jarvis_home.clone())  // Passa o diretório para o core
    .build()
    .await?;

// O jarvis-core gerencia tudo automaticamente:
// - Lê/escreve threads em rollout-*.jsonl
// - Lê configuração de config.toml
// - Gerencia credenciais
```

---

## 🚀 Para Testar Localmente

### **Opção 1: Rodar Direto (Recomendado para Testes)**

**NÃO precisa de Docker Compose!** Você pode rodar direto:

```bash
# 1. Compilar
cd jarvis-rs
cargo build --package jarvis-web-api --release

# 2. Configurar JARVIS_HOME (opcional, padrão é ~/.jarvis)
export JARVIS_HOME=~/.jarvis  # Linux/Mac
# ou
$env:JARVIS_HOME = "$HOME\.jarvis"  # Windows PowerShell

# 3. Criar config.toml se não existir
# O jarvis-core criará automaticamente na primeira execução

# 4. Rodar
./target/release/jarvis-web-api
```

### **Opção 2: Docker Compose (Opcional)**

Se quiser isolar o ambiente, pode criar um `docker-compose.yml`:

```yaml
version: '3.8'

services:
  web-api:
    build:
      context: .
      dockerfile: jarvis-rs/web-api/Dockerfile
    ports:
      - "3000:3000"
    environment:
      - JARVIS_HOME=/app/.jarvis
      - JARVIS_API_KEY=sua-api-key-aqui
      - RUST_LOG=info
    volumes:
      # Persistir dados do Jarvis
      - jarvis-data:/app/.jarvis
    restart: unless-stopped

volumes:
  jarvis-data:
```

**Mas não é obrigatório!** O web-api funciona perfeitamente sem Docker.

---

## ✅ O que o web-api já faz automaticamente

### **1. Inicialização do Core**

```rust
// server.rs - Linhas 72-101
pub async fn run_server(...) {
    // 1. Encontra jarvis_home
    let jarvis_home = find_jarvis_home()?;
    
    // 2. Carrega configuração
    let config = ConfigBuilder::default()
        .jarvis_home(jarvis_home.clone())
        .build()
        .await?;
    
    // 3. Inicializa serviços do core
    let auth_manager = AuthManager::new(...);
    let models_manager = ModelsManager::new(...);
    
    // 4. Tudo pronto para usar!
}
```

### **2. Acesso a Threads**

```rust
// handlers/threads.rs - Linha 114
let jarvis_home = &state.config.jarvis_home;

// Usa RolloutRecorder do core para listar threads
RolloutRecorder::list_threads(jarvis_home, ...)
```

### **3. Criação/Resumo de Threads**

```rust
// handlers/chat.rs - Linha 135
let thread_manager = ThreadManager::new(
    state.config.jarvis_home.clone(),
    state.auth_manager.clone(),
    SessionSource::Exec,
);

// Cria ou resume thread automaticamente
thread_manager.start_thread(...)
// ou
thread_manager.resume_thread_from_rollout(...)
```

---

## 🔧 Configuração Necessária

### **Mínimo Necessário**

1. **JARVIS_HOME** (ou usar padrão `~/.jarvis`)
2. **config.toml** (criado automaticamente na primeira execução)
3. **API Key** (para autenticação da API)

### **Exemplo de config.toml**

```toml
[api]
api_key = "sua-api-key-aqui"
port = 3000
bind_address = "0.0.0.0"
enable_cors = true

[model_provider]
# Configuração do modelo LLM
# (já configurado no seu projeto)
```

---

## 🧪 Testar Localmente (Passo a Passo)

### **1. Compilar**

```bash
cd jarvis-rs
cargo build --package jarvis-web-api --release
```

### **2. Configurar (Primeira Vez)**

```bash
# Criar diretório se não existir
mkdir -p ~/.jarvis

# Criar config.toml básico
cat > ~/.jarvis/config.toml << EOF
[api]
api_key = "$(openssl rand -hex 32)"
port = 3000
bind_address = "127.0.0.1"
enable_cors = true
EOF
```

### **3. Rodar**

```bash
# Linux/Mac
./target/release/jarvis-web-api

# Windows
.\target\release\jarvis-web-api.exe
```

### **4. Testar**

```bash
# Health check
curl http://localhost:3000/api/health

# Interface web
# Abra: http://localhost:3000
```

---

## 🐳 Docker Compose (Opcional)

Se quiser usar Docker Compose para isolar o ambiente:

### **Criar docker-compose.yml**

```yaml
version: '3.8'

services:
  jarvis-web-api:
    build:
      context: .
      dockerfile: jarvis-rs/web-api/Dockerfile
    ports:
      - "3000:3000"
    environment:
      - JARVIS_HOME=/app/.jarvis
      - JARVIS_API_KEY=${JARVIS_API_KEY:-change-me}
      - RUST_LOG=info
    volumes:
      - jarvis-data:/app/.jarvis
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:3000/api/health"]
      interval: 30s
      timeout: 10s
      retries: 3

volumes:
  jarvis-data:
```

### **Usar**

```bash
# Gerar API key
export JARVIS_API_KEY=$(openssl rand -hex 32)

# Subir
docker-compose up -d

# Ver logs
docker-compose logs -f

# Parar
docker-compose down
```

---

## 📊 Resumo

| Aspecto | Detalhes |
|---------|----------|
| **Banco de Dados** | SQLite (arquivos em disco) |
| **Localização** | `~/.jarvis` (ou `JARVIS_HOME`) |
| **Formato** | Arquivos JSONL (rollout-*.jsonl) |
| **Integração** | Automática via `jarvis-core` |
| **Configuração** | Automática (cria na primeira execução) |
| **Docker Compose** | Opcional (não obrigatório) |
| **Teste Local** | Simples: compilar e rodar |

---

## ✅ Conclusão

**Você NÃO precisa:**
- ❌ Configurar banco de dados separado
- ❌ Levantar Docker Compose obrigatoriamente
- ❌ Configurar conexões de banco
- ❌ Gerenciar migrations

**Você só precisa:**
- ✅ Compilar o projeto
- ✅ Ter `jarvis_home` configurado (ou usar padrão)
- ✅ Ter `config.toml` (criado automaticamente)

**Tudo funciona automaticamente!** 🎉
