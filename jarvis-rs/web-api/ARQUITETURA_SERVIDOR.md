# 🏗️ Arquitetura do Servidor Web API

## 📌 Resumo: **UM ÚNICO SERVIÇO**

O `jarvis-web-api` é um **servidor único** que serve **tanto a API REST quanto a interface web** na mesma porta.

---

## 🔄 Como Funciona

### **Estrutura do Servidor**

```
┌─────────────────────────────────────────┐
│     jarvis-web-api (Servidor Axum)      │
│         Porta: 3000 (ou PORT)           │
└─────────────────────────────────────────┘
              │
              ├─── /api/* ────→ API REST (JSON)
              │    ├── /api/health
              │    ├── /api/chat
              │    ├── /api/threads
              │    └── /api/config
              │
              └─── /* ─────────→ Interface Web (HTML)
                   └── / (index.html)
```

### **Rotas Configuradas**

#### 1. **API REST** (`/api/*`)

```rust
// server.rs - Linhas 23-26
.route("/api/health", GET)    // Health check
.route("/api/chat", POST)      // Chat com Jarvis
.route("/api/threads", GET)    // Listar threads
.route("/api/config", GET)     // Configuração
```

- **Autenticação**: Requer `Authorization: Bearer <api_key>`
- **Resposta**: JSON
- **Uso**: Chamadas programáticas, mobile apps, etc.

#### 2. **Interface Web** (`/`)

```rust
// server.rs - Linhas 47-67
// Serve arquivos estáticos da pasta static/
router.nest_service("/", ServeDir::new("static"))
```

- **Autenticação**: Nenhuma (arquivos estáticos)
- **Resposta**: HTML, CSS, JS
- **Uso**: Navegador web, interface visual

---

## 🎯 Exemplo Prático

### **Cenário 1: Acessar Interface Web**

```
Usuário abre navegador:
  → https://seu-projeto.up.railway.app/
  → Servidor retorna: static/index.html
  → HTML carrega HTMX e Tailwind CSS
  → Interface web interativa aparece
```

### **Cenário 2: Usar API REST**

```bash
# Chamada direta à API
curl -X POST https://seu-projeto.up.railway.app/api/chat \
  -H "Authorization: Bearer sua-api-key" \
  -H "Content-Type: application/json" \
  -d '{"prompt": "Olá!"}'
```

### **Cenário 3: Interface Web chama API**

```
Interface Web (index.html):
  → Usuário digita mensagem
  → HTMX faz POST para /api/chat
  → Servidor processa (mesmo endpoint da API REST)
  → Retorna HTML fragment (para HTMX) ou JSON
```

---

## 🔐 Autenticação

### **Middleware de Autenticação**

```rust
// middleware/auth.rs
pub async fn validate_auth(...) {
    // Skip auth para:
    // 1. /api/health (health check público)
    // 2. Arquivos estáticos (interface web)
    
    // Requer auth para:
    // - Todos os outros /api/* endpoints
}
```

### **Como Funciona**

1. **Interface Web (`/`)**: 
   - ✅ Sem autenticação (arquivos estáticos)
   - ⚠️ Mas a interface web precisa da API key para chamar `/api/chat`

2. **API REST (`/api/*`)**:
   - ✅ `/api/health`: Sem autenticação
   - 🔒 `/api/chat`, `/api/threads`, `/api/config`: Requerem `Authorization: Bearer <key>`

---

## 📁 Estrutura de Arquivos

```
jarvis-rs/web-api/
├── src/
│   ├── main.rs          # Entry point
│   ├── server.rs        # Configuração do servidor Axum
│   ├── handlers/        # Handlers da API REST
│   │   ├── chat.rs      # POST /api/chat
│   │   ├── threads.rs   # GET /api/threads
│   │   ├── config.rs    # GET /api/config
│   │   └── health.rs    # GET /api/health
│   └── middleware/
│       └── auth.rs       # Middleware de autenticação
│
└── static/              # Arquivos estáticos (interface web)
    └── index.html       # Interface web principal
```

---

## 🚀 Deploy no Railway

### **Um Único Serviço**

No Railway, você precisa criar **apenas 1 serviço**:

```yaml
# railway.json
{
  "build": {
    "builder": "DOCKERFILE",
    "dockerfilePath": "jarvis-rs/web-api/Dockerfile"
  },
  "deploy": {
    "startCommand": "/app/jarvis-web-api",
    "healthcheckPath": "/api/health"
  }
}
```

### **Variáveis de Ambiente**

```bash
JARVIS_API_KEY=sua-key-aqui  # Obrigatório
RUST_LOG=info                # Opcional
PORT=3000                     # Railway define automaticamente
```

---

## ✅ Vantagens Desta Arquitetura

1. **Simplicidade**: Um único processo, uma única porta
2. **Eficiência**: Menos recursos, menos complexidade
3. **Manutenção**: Tudo em um lugar
4. **Deploy**: Mais fácil (um único serviço)
5. **Custo**: Menor (um único container/serviço)

---

## 🔄 Fluxo Completo

### **1. Usuário acessa interface web**

```
Browser → GET / 
         → Servidor retorna index.html
         → HTML carrega HTMX
         → Interface pronta
```

### **2. Usuário envia mensagem**

```
Browser → POST /api/chat (com Authorization header)
         → Middleware valida API key
         → Handler processa mensagem
         → Retorna HTML fragment (para HTMX)
         → Interface atualiza automaticamente
```

### **3. Acesso via API direta**

```
Cliente → POST /api/chat (com Authorization header)
         → Middleware valida API key
         → Handler processa mensagem
         → Retorna JSON
```

---

## 📊 Resumo

| Aspecto | Detalhes |
|--------|----------|
| **Serviços** | 1 único serviço |
| **Porta** | 1 única porta (3000 ou PORT) |
| **API REST** | `/api/*` endpoints |
| **Interface Web** | `/` (arquivos estáticos) |
| **Autenticação** | API Key para `/api/*` (exceto `/api/health`) |
| **Deploy** | 1 único container/serviço no Railway |

---

## 🎯 Conclusão

**Você NÃO precisa de dois serviços separados!**

O `jarvis-web-api` já serve:
- ✅ API REST (`/api/*`)
- ✅ Interface Web (`/`)

Tudo no mesmo servidor, na mesma porta. Basta fazer deploy de **um único serviço** no Railway.
