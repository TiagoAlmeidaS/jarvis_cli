# 🐳 Docker Compose - Guia Completo

## 🚀 Início Rápido

### **Opção 1: Script Automático (Recomendado)**

```bash
# Linux/Mac
./scripts/start-web-api.sh

# Windows PowerShell
.\scripts\start-web-api.ps1
```

O script vai:
- ✅ Criar `.env` automaticamente se não existir
- ✅ Gerar API key automaticamente
- ✅ Subir o container
- ✅ Verificar se está funcionando

### **Opção 2: Manual**

```bash
# 1. Criar .env (se não existir)
cp .env.example .env

# 2. Editar .env e adicionar sua API key
# Gere uma key: openssl rand -hex 32

# 3. Subir o container
docker compose -f docker-compose.web-api.yml up -d

# 4. Ver logs
docker compose -f docker-compose.web-api.yml logs -f
```

---

## 📋 Comandos Úteis

### **Subir o serviço**

```bash
docker compose -f docker-compose.web-api.yml up -d
```

### **Ver logs**

```bash
docker compose -f docker-compose.web-api.yml logs -f
```

### **Parar o serviço**

```bash
docker compose -f docker-compose.web-api.yml down
```

### **Parar e remover volumes (limpar dados)**

```bash
docker compose -f docker-compose.web-api.yml down -v
```

### **Reconstruir a imagem**

```bash
docker compose -f docker-compose.web-api.yml build --no-cache
docker compose -f docker-compose.web-api.yml up -d
```

### **Ver status**

```bash
docker compose -f docker-compose.web-api.yml ps
```

### **Executar comando no container**

```bash
docker exec -it jarvis-web-api bash
```

---

## 🔧 Configuração

### **Variáveis de Ambiente (.env)**

Crie um arquivo `.env` na raiz do projeto:

```bash
# API Key (OBRIGATÓRIO)
JARVIS_API_KEY=sua-api-key-aqui

# Porta (opcional, padrão: 3000)
WEB_API_PORT=3000

# Log level (opcional, padrão: info)
RUST_LOG=info
```

### **Gerar API Key**

```bash
# Linux/Mac
openssl rand -hex 32

# Windows PowerShell
-join ((65..90) + (97..122) + (48..57) | Get-Random -Count 32 | ForEach-Object {[char]$_}))
```

---

## 🧪 Testar

### **Health Check**

```bash
curl http://localhost:3000/api/health
```

### **Interface Web**

Abra no navegador: http://localhost:3000

### **API REST**

```bash
# Substitua SUA_API_KEY pela key do .env
curl -X POST http://localhost:3000/api/chat \
  -H "Authorization: Bearer SUA_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"prompt": "Olá, Jarvis!"}'
```

---

## 📊 Estrutura

```
projeto/
├── docker-compose.web-api.yml  # Configuração Docker Compose
├── .env                        # Variáveis de ambiente (não commitar!)
├── .env.example                # Exemplo de .env
├── scripts/
│   ├── start-web-api.sh        # Script Linux/Mac
│   └── start-web-api.ps1       # Script Windows
└── jarvis-rs/
    └── web-api/
        └── Dockerfile           # Dockerfile do serviço
```

---

## 💾 Persistência de Dados

Os dados do Jarvis são persistidos em um volume Docker:

- **Volume**: `jarvis_web_api_data`
- **Localização no container**: `/app/.jarvis`
- **Conteúdo**: config.toml, threads (rollout-*.jsonl), credenciais

### **Backup**

```bash
# Criar backup do volume
docker run --rm -v jarvis_web_api_data:/data -v $(pwd):/backup \
  alpine tar czf /backup/jarvis-backup-$(date +%Y%m%d).tar.gz -C /data .

# Restaurar backup
docker run --rm -v jarvis_web_api_data:/data -v $(pwd):/backup \
  alpine tar xzf /backup/jarvis-backup-YYYYMMDD.tar.gz -C /data
```

---

## 🐛 Troubleshooting

### **Container não inicia**

```bash
# Ver logs detalhados
docker compose -f docker-compose.web-api.yml logs

# Verificar se a porta está em uso
netstat -an | grep 3000  # Linux/Mac
netstat -an | findstr 3000  # Windows
```

### **Health check falhando**

```bash
# Verificar logs do container
docker logs jarvis-web-api

# Verificar se o serviço está rodando dentro do container
docker exec jarvis-web-api wget -O- http://localhost:3000/api/health
```

### **Erro de permissão**

```bash
# Verificar permissões do volume
docker volume inspect jarvis_web_api_data
```

### **Reconstruir do zero**

```bash
# Parar e remover tudo
docker compose -f docker-compose.web-api.yml down -v

# Reconstruir
docker compose -f docker-compose.web-api.yml build --no-cache

# Subir novamente
docker compose -f docker-compose.web-api.yml up -d
```

---

## 🔒 Segurança

### **⚠️ IMPORTANTE**

1. **Nunca commite o arquivo `.env`** no Git
2. **Use API keys fortes** (32+ caracteres aleatórios)
3. **Mantenha o Docker atualizado**
4. **Use HTTPS em produção** (via reverse proxy)

### **.gitignore**

Certifique-se de que `.env` está no `.gitignore`:

```
.env
.env.local
```

---

## 📚 Próximos Passos

- Veja [README.md](./README.md) para documentação completa da API
- Veja [DEPLOY_RAILWAY.md](./DEPLOY_RAILWAY.md) para deploy em produção
- Veja [TESTE_LOCAL.md](./TESTE_LOCAL.md) para testes sem Docker

---

## ✅ Checklist

- [ ] Docker instalado e rodando
- [ ] Arquivo `.env` criado com `JARVIS_API_KEY`
- [ ] Container subido: `docker compose -f docker-compose.web-api.yml up -d`
- [ ] Health check passando: `curl http://localhost:3000/api/health`
- [ ] Interface web acessível: http://localhost:3000
- [ ] API funcionando: teste com `curl` ou interface web
