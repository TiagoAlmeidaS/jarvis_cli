# API Web do Jarvis

## Visão Geral

A API Web do Jarvis permite acesso remoto ao sistema via HTTP REST, seguindo a arquitetura hexagonal. Ela reutiliza o `core` existente, permitindo interação com o Jarvis através de uma interface web ou chamadas HTTP diretas.

## Configuração

### Config.toml

Adicione a seguinte seção ao seu `~/.jarvis/config.toml`:

```toml
[api]
# API key para autenticação (obrigatório)
api_key = "seu-token-secreto-aqui"

# Porta do servidor (padrão: 3000)
port = 3000

# Endereço de bind (padrão: 0.0.0.0)
bind_address = "0.0.0.0"

# Habilitar CORS para desenvolvimento (padrão: false)
enable_cors = false
```

### Geração de API Key

Recomenda-se usar um token seguro. Você pode gerar um usando:

```bash
# Linux/Mac
openssl rand -hex 32

# Ou usando Python
python3 -c "import secrets; print(secrets.token_urlsafe(32))"
```

## Executando o Servidor

### Compilação

```bash
cd jarvis-rs
cargo build --package jarvis-web-api --release
```

### Execução

```bash
./target/release/jarvis-web-api
```

O servidor estará disponível em `http://0.0.0.0:3000` (ou na porta configurada).

## Endpoints

### GET /api/health

Health check do servidor.

**Resposta:**
```json
{
  "status": "ok",
  "version": "0.0.0"
}
```

### POST /api/chat

Envia uma mensagem para o Jarvis e recebe uma resposta.

**Autenticação:** Requer header `Authorization: Bearer <api_key>`

**Content-Type:** 
- `application/json` para requisições JSON
- `application/x-www-form-urlencoded` para formulários HTML

**Request (JSON):**
```json
{
  "prompt": "Explique o que é Rust",
  "thread_id": "550e8400-e29b-41d4-a716-446655440000"
}
```

**Validação:**
- `prompt`: Obrigatório, mínimo 1 caractere, máximo 100.000 caracteres
- `thread_id`: Opcional, deve ser um UUID válido se fornecido

**Request (Form):**
```
prompt=Explique o que é Rust&thread_id=opcional-thread-id
```

**Response (JSON):**
```json
{
  "reply": "Rust é uma linguagem de programação...",
  "thread_id": "thread-123"
}
```

**Response (HTML):**
Retorna um fragmento HTML para uso com HTMX:
```html
<div class="message space-y-2">
  <div class="bg-gray-700 rounded-lg p-3">
    <p class="text-sm text-gray-400">Você:</p>
    <p class="text-gray-100">Explique o que é Rust</p>
  </div>
  <div class="bg-blue-600 rounded-lg p-3">
    <p class="text-sm text-blue-200">Jarvis:</p>
    <p class="text-white">Rust é uma linguagem de programação...</p>
  </div>
</div>
```

### GET /api/threads

Lista threads disponíveis com paginação.

**Autenticação:** Requer header `Authorization: Bearer <api_key>`

**Query Parameters:**
- `limit` (opcional): Número máximo de threads a retornar (padrão: 20)
- `cursor` (opcional): Token de paginação para continuar a partir de uma posição
- `sort` (opcional): Ordenação - `created_at` (padrão) ou `updated_at`

**Response:**
```json
{
  "threads": [
    {
      "id": "550e8400-e29b-41d4-a716-446655440000",
      "preview": "Explique o que é Rust...",
      "created_at": "2024-01-15T10:30:00Z",
      "updated_at": "2024-01-15T11:45:00Z"
    }
  ],
  "next_cursor": "2024-01-15T10:30:00Z|550e8400-e29b-41d4-a716-446655440000"
}
```

### GET /api/config

Retorna configuração atual (sem secrets).

**Autenticação:** Requer header `Authorization: Bearer <api_key>`

**Response:**
```json
{
  "model_provider": "databricks",
  "model": "databricks-claude-opus-4-5",
  "port": 3000,
  "bind_address": "0.0.0.0"
}
```

## Interface Web

Acesse `http://localhost:3000/` para usar a interface web integrada.

A interface usa:
- **HTMX** para interações sem recarregar página
- **Tailwind CSS** para estilização
- **JavaScript mínimo** para funcionalidades básicas

## Autenticação

Todos os endpoints (exceto `/api/health` e arquivos estáticos) requerem autenticação via Bearer Token:

```bash
curl -H "Authorization: Bearer seu-token-aqui" \
     -H "Content-Type: application/json" \
     -d '{"prompt": "Olá!"}' \
     http://localhost:3000/api/chat
```

## Exemplos de Uso

### cURL

```bash
# Health check
curl http://localhost:3000/api/health

# Chat (JSON)
curl -X POST \
     -H "Authorization: Bearer seu-token" \
     -H "Content-Type: application/json" \
     -d '{"prompt": "Crie uma função em Rust"}' \
     http://localhost:3000/api/chat

# Chat com thread_id (continuar conversa)
curl -X POST \
     -H "Authorization: Bearer seu-token" \
     -H "Content-Type: application/json" \
     -d '{"prompt": "Continue a conversa", "thread_id": "uuid-da-thread"}' \
     http://localhost:3000/api/chat

# Listar threads
curl -X GET \
     -H "Authorization: Bearer seu-token" \
     "http://localhost:3000/api/threads?limit=10&sort=updated_at"

# Chat (Form)
curl -X POST \
     -H "Authorization: Bearer seu-token" \
     -H "Content-Type: application/x-www-form-urlencoded" \
     -d "prompt=Crie uma função em Rust" \
     http://localhost:3000/api/chat
```

### Python

```python
import requests

API_URL = "http://localhost:3000"
API_KEY = "seu-token-aqui"

headers = {
    "Authorization": f"Bearer {API_KEY}",
    "Content-Type": "application/json"
}

# Enviar mensagem
response = requests.post(
    f"{API_URL}/api/chat",
    headers=headers,
    json={"prompt": "Explique o que é Rust"}
)

data = response.json()
print(f"Resposta: {data['reply']}")
print(f"Thread ID: {data['thread_id']}")
```

### JavaScript (Fetch)

```javascript
const API_URL = 'http://localhost:3000';
const API_KEY = 'seu-token-aqui';

async function chat(prompt) {
  const response = await fetch(`${API_URL}/api/chat`, {
    method: 'POST',
    headers: {
      'Authorization': `Bearer ${API_KEY}`,
      'Content-Type': 'application/json'
    },
    body: JSON.stringify({ prompt })
  });
  
  const data = await response.json();
  return data;
}

// Uso
chat("Explique o que é Rust").then(data => {
  console.log(data.reply);
});
```

## Deploy em VPS

### 1. Compilar para Linux

```bash
# No seu ambiente de desenvolvimento
cargo build --package jarvis-web-api --release --target x86_64-unknown-linux-musl
```

### 2. Transferir para VPS

```bash
scp target/x86_64-unknown-linux-musl/release/jarvis-web-api user@vps:/opt/jarvis/
```

### 3. Configurar systemd

Crie `/etc/systemd/system/jarvis-web-api.service`:

```ini
[Unit]
Description=Jarvis API Server
After=network.target

[Service]
Type=simple
User=jarvis
WorkingDirectory=/opt/jarvis
ExecStart=/opt/jarvis/jarvis-web-api
Restart=always
RestartSec=10
Environment="RUST_LOG=info"

[Install]
WantedBy=multi-user.target
```

### 4. Iniciar serviço

```bash
sudo systemctl daemon-reload
sudo systemctl enable jarvis-web-api
sudo systemctl start jarvis-web-api
```

### 5. Configurar Nginx (Opcional)

```nginx
server {
    listen 80;
    server_name jarvis.seu-dominio.com;

    location / {
        proxy_pass http://localhost:3000;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection 'upgrade';
        proxy_set_header Host $host;
        proxy_cache_bypass $http_upgrade;
    }
}
```

## Segurança

### Recomendações

1. **Use HTTPS em produção**: Configure um reverse proxy (Nginx/Caddy) com certificado SSL
2. **API Key forte**: Use tokens longos e aleatórios
3. **Firewall**: Restrinja acesso à porta da API apenas para IPs confiáveis
4. **Rate Limiting**: Considere adicionar rate limiting (futuro)
5. **CORS**: Mantenha `enable_cors = false` em produção a menos que necessário

### Variáveis de Ambiente

Você pode sobrescrever a API key via variável de ambiente:

```bash
export JARVIS_API_KEY="seu-token"
```

## Troubleshooting

### Erro: "API not configured"

Verifique se a seção `[api]` existe no `config.toml` e se `api_key` está definido.

### Erro: "Invalid API key"

Verifique se o token no header `Authorization: Bearer <token>` corresponde ao `api_key` no `config.toml`.

### Erro: "Prompt too long"

O prompt excedeu o limite de 100.000 caracteres. Reduza o tamanho da mensagem.

### Erro: "Invalid thread_id format"

O `thread_id` fornecido não é um UUID válido. Use o formato: `550e8400-e29b-41d4-a716-446655440000`.

### Erro: "Thread not found"

A thread especificada não existe ou foi arquivada. Verifique o ID ou crie uma nova thread omitindo o `thread_id`.

### Servidor não inicia

Verifique:
- Se a porta está disponível
- Se o `bind_address` está correto
- Logs do servidor para mais detalhes
- Permissões de leitura/escrita no diretório `jarvis_home`

### Arquivos estáticos não carregam

Os arquivos estáticos devem estar no diretório `static/` relativo ao executável. Em produção, considere servir via Nginx.

### Thread não resume corretamente

- Verifique se a thread existe usando `GET /api/threads`
- Certifique-se de que o `thread_id` está no formato UUID correto
- Verifique os logs do servidor para erros específicos

## Evolução Futura

- [ ] WebSockets para streaming real-time
- [ ] JWT para múltiplos usuários
- [ ] Rate limiting por IP/token
- [ ] Métricas e observabilidade
- [ ] Suporte a upload de arquivos
- [ ] Streaming de respostas (Server-Sent Events)

## Arquitetura

A API Web segue a arquitetura hexagonal:

```
┌──────────┐  ┌──────────┐  ┌──────────┐
│   CLI    │  │  Daemon  │  │    API   │
└────┬─────┘  └────┬─────┘  └────┬─────┘
     │             │              │
     └─────────────┴──────────────┘
                   │
            ┌──────▼──────┐
            │    Core     │
            │ (LLM, DB,   │
            │  Session)   │
            └─────────────┘
```

Todos os componentes (CLI, Daemon, API) compartilham o mesmo `core`, garantindo consistência e reutilização de código.
