# Jarvis Web API

Servidor HTTP REST para acessar o Jarvis remotamente via web interface ou chamadas HTTP.

## Compilação

```bash
cd jarvis-rs
cargo build --package jarvis-web-api --release
```

O binário estará em `target/release/jarvis-web-api`.

## Configuração

Adicione a seção `[api]` ao seu `~/.jarvis/config.toml`:

```toml
[api]
api_key = "seu-token-secreto-aqui"  # Obrigatório
port = 3000                          # Opcional, padrão: 3000
bind_address = "0.0.0.0"            # Opcional, padrão: 0.0.0.0
enable_cors = false                 # Opcional, padrão: false
```

### Gerar API Key

```bash
# Linux/Mac
openssl rand -hex 32

# Windows (PowerShell)
[Convert]::ToBase64String((1..32 | ForEach-Object { Get-Random -Minimum 0 -Maximum 256 }))
```

## Execução

```bash
./target/release/jarvis-web-api
```

O servidor estará disponível em `http://localhost:3000`.

## Endpoints

### Interface Web
- `GET /` - Interface web (HTMX + Tailwind CSS)

### API REST
- `GET /api/health` - Health check (sem autenticação)
- `GET /api/config` - Obter configuração (requer autenticação)
- `GET /api/threads` - Listar threads (requer autenticação)
- `POST /api/chat` - Enviar mensagem ao Jarvis (requer autenticação)

### Autenticação

Todas as requisições para `/api/*` (exceto `/api/health`) requerem o header:

```
Authorization: Bearer <sua-api-key>
```

### Exemplo de Uso

```bash
# Health check (sem autenticação)
curl http://localhost:3000/api/health

# Chat (com autenticação)
curl -X POST http://localhost:3000/api/chat \
  -H "Authorization: Bearer sua-api-key" \
  -H "Content-Type: application/json" \
  -d '{"prompt": "Olá, Jarvis!"}'
```

## Interface Web

Acesse `http://localhost:3000` no navegador para usar a interface web interativa.

## Documentação Completa

Veja `docs/api-web.md` para documentação completa, incluindo:
- Arquitetura
- Deploy em VPS
- Configuração de proxy reverso (Nginx/Caddy)
- Systemd service
- Segurança e HTTPS
