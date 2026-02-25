# 🚀 Guia de Deploy - Jarvis Web API

Este guia mostra como fazer deploy da API Web do Jarvis em serviços gratuitos.

## 📋 Pré-requisitos

1. Conta em um dos serviços:
   - [Render.com](https://render.com) (Recomendado - mais fácil)
   - [Railway.app](https://railway.app)
   - [Fly.io](https://fly.io)

2. Repositório Git configurado (GitHub, GitLab, etc.)

3. API Key gerada:
   ```bash
   # Linux/Mac
   openssl rand -hex 32
   
   # Windows PowerShell
   [Convert]::ToBase64String((1..32 | ForEach-Object { Get-Random -Minimum 0 -Maximum 256 }))
   ```

---

## 🎯 Opção 1: Render.com (Recomendado - Mais Fácil)

### Passo 1: Preparar Repositório

1. Certifique-se de que o código está no GitHub/GitLab
2. O arquivo `render.yaml` já está configurado

### Passo 2: Deploy no Render

1. Acesse [Render Dashboard](https://dashboard.render.com)
2. Clique em "New +" → "Blueprint"
3. Conecte seu repositório
4. Render detectará automaticamente o `render.yaml`
5. Configure as variáveis de ambiente:
   - `JARVIS_API_KEY`: Sua API key gerada
   - `RUST_LOG`: `info` (opcional)
6. Clique em "Apply"

### Passo 3: Configurar API Key no config.toml

O Render não persiste arquivos no sistema de arquivos. Você precisa:

**Opção A**: Usar apenas variável de ambiente `JARVIS_API_KEY`
- A API já suporta isso via `config.toml` ou env var

**Opção B**: Criar `config.toml` via script de inicialização

### Limitações do Plano Gratuito Render:
- ✅ 750 horas/mês (suficiente para uso pessoal)
- ✅ Auto-sleep após 15min de inatividade
- ✅ HTTPS automático
- ⚠️ Wake-up pode levar alguns segundos

---

## 🚂 Opção 2: Railway.app

### Passo 1: Instalar Railway CLI

```bash
# Windows (PowerShell)
iwr https://railway.app/install.ps1 | iex

# Linux/Mac
curl -fsSL https://railway.app/install.sh | sh
```

### Passo 2: Deploy

```bash
# Login
railway login

# No diretório do projeto
cd jarvis-rs/web-api

# Inicializar projeto
railway init

# Adicionar variáveis de ambiente
railway variables set JARVIS_API_KEY="sua-api-key-aqui"
railway variables set RUST_LOG=info

# Deploy
railway up
```

### Limitações do Plano Gratuito Railway:
- ✅ $5 créditos/mês
- ✅ Sem auto-sleep
- ✅ HTTPS automático
- ⚠️ Créditos podem acabar com uso intenso

---

## ✈️ Opção 3: Fly.io

### Passo 1: Instalar Fly CLI

```bash
# Windows (PowerShell)
powershell -Command "iwr https://fly.io/install.ps1 -useb | iex"

# Linux/Mac
curl -L https://fly.io/install.sh | sh
```

### Passo 2: Deploy

```bash
# Login
flyctl auth login

# No diretório do projeto
cd jarvis-rs/web-api

# Primeira vez: criar app
flyctl launch

# Configurar secrets
flyctl secrets set JARVIS_API_KEY="sua-api-key-aqui"
flyctl secrets set RUST_LOG=info

# Deploy
flyctl deploy
```

### Limitações do Plano Gratuito Fly.io:
- ✅ 3 VMs compartilhadas
- ✅ 3GB de storage
- ✅ 160GB de transferência/mês
- ✅ HTTPS automático

---

## 🧪 Testar Localmente Primeiro

Antes de fazer deploy, teste localmente:

### 1. Compilar

```bash
cd jarvis-rs
cargo build --package jarvis-web-api --release
```

### 2. Configurar

Crie `~/.jarvis/config.toml`:

```toml
[api]
api_key = "sua-api-key-aqui"
port = 3000
bind_address = "0.0.0.0"
enable_cors = true  # Para testar localmente
```

### 3. Executar

```bash
./target/release/jarvis-web-api
```

### 4. Testar

```bash
# Health check
curl http://localhost:3000/api/health

# Chat (substitua sua-api-key-aqui)
curl -X POST http://localhost:3000/api/chat \
  -H "Authorization: Bearer sua-api-key-aqui" \
  -H "Content-Type: application/json" \
  -d '{"prompt": "Olá, Jarvis!"}'
```

---

## 🔧 Configuração Avançada

### Variáveis de Ambiente Suportadas

- `PORT`: Porta do servidor (padrão: 3000)
- `JARVIS_HOME`: Diretório do Jarvis (padrão: `~/.jarvis`)
- `JARVIS_API_KEY`: API Key (sobrescreve config.toml)
- `RUST_LOG`: Nível de logging (info, debug, warn, error)

### Configuração via config.toml

Em serviços cloud, você pode criar o `config.toml` programaticamente:

```bash
# No Dockerfile ou script de inicialização
mkdir -p /app/.jarvis
cat > /app/.jarvis/config.toml << EOF
[api]
api_key = "${JARVIS_API_KEY}"
port = ${PORT:-3000}
bind_address = "0.0.0.0"
enable_cors = true
EOF
```

---

## 🐳 Deploy com Docker (Local ou VPS)

### Build

```bash
cd jarvis-rs
docker build -f web-api/Dockerfile -t jarvis-web-api .
```

### Run

```bash
docker run -d \
  -p 3000:3000 \
  -e JARVIS_API_KEY="sua-api-key" \
  -e PORT=3000 \
  -v $(pwd)/.jarvis:/app/.jarvis \
  --name jarvis-api \
  jarvis-web-api
```

---

## ✅ Checklist de Deploy

- [ ] Código commitado no Git
- [ ] API Key gerada e segura
- [ ] Testado localmente
- [ ] Variáveis de ambiente configuradas
- [ ] Health check funcionando (`/api/health`)
- [ ] HTTPS configurado (automático na maioria dos serviços)
- [ ] Testado acesso remoto

---

## 🆘 Troubleshooting

### Erro: "API not configured"
- Verifique se `JARVIS_API_KEY` está definida
- Verifique se `config.toml` existe e tem a seção `[api]`

### Erro: "Port already in use"
- Verifique se a variável `PORT` está correta
- Alguns serviços definem `PORT` automaticamente

### Erro: "Failed to find jarvis home"
- Verifique se `JARVIS_HOME` está definida
- Certifique-se de que o diretório existe e tem permissões

### App não inicia
- Verifique logs: `railway logs` ou `flyctl logs`
- Verifique se todas as dependências estão no Dockerfile
- Teste localmente primeiro

---

## 📚 Próximos Passos

Após deploy bem-sucedido:
1. Testar acesso remoto
2. Configurar domínio personalizado (opcional)
3. Adicionar monitoramento básico
4. Configurar backups (se necessário)

---

## 🔗 Links Úteis

- [Documentação da API](./README.md)
- [Status do Plano](./PLANO_STATUS.md)
- [Próximas Etapas](./PROXIMAS_ETAPAS.md)
