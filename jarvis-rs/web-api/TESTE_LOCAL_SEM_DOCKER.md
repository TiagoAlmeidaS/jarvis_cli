# 🧪 Teste Local sem Docker - Guia Rápido

## 🚀 Início Rápido

### **Windows (PowerShell)**

```powershell
# 1. Iniciar o servidor
.\scripts\test-local-web-api.ps1

# 2. Em outro terminal, testar a API
.\scripts\test-api.ps1
```

### **Linux/Mac (Bash)**

```bash
# 1. Dar permissão de execução
chmod +x scripts/test-local-web-api.sh scripts/test-api.sh

# 2. Iniciar o servidor
./scripts/test-local-web-api.sh

# 3. Em outro terminal, testar a API
./scripts/test-api.sh
```

---

## 📋 O que os scripts fazem

### **test-local-web-api.ps1 / test-local-web-api.sh**

1. ✅ Verifica se Rust está instalado
2. ✅ Compila o projeto (`cargo build --release`)
3. ✅ Cria `~/.jarvis/config.toml` se não existir
4. ✅ Gera API key automaticamente
5. ✅ Inicia o servidor na porta 3000

### **test-api.ps1 / test-api.sh**

1. ✅ Testa `/api/health` (health check)
2. ✅ Testa `/api/config` (com autenticação)
3. ✅ Testa `/api/chat` (enviar mensagem)
4. ✅ Testa interface web (`/`)

---

## 🔧 Configuração Manual

Se preferir fazer manualmente:

### **1. Compilar**

```bash
cd jarvis-rs
cargo build --package jarvis-web-api --release
```

### **2. Configurar**

```bash
# Criar diretório
mkdir -p ~/.jarvis  # Linux/Mac
# ou
New-Item -ItemType Directory -Path "$env:USERPROFILE\.jarvis"  # Windows

# Criar config.toml
cat > ~/.jarvis/config.toml << EOF
[api]
api_key = "$(openssl rand -hex 32)"
port = 3000
bind_address = "127.0.0.1"
enable_cors = true
EOF
```

### **3. Executar**

```bash
# Linux/Mac
./target/release/jarvis-web-api

# Windows
.\target\release\jarvis-web-api.exe
```

---

## 🧪 Testar Manualmente

### **Health Check**

```bash
curl http://localhost:3000/api/health
```

### **Config (com autenticação)**

```bash
# Substitua SUA_API_KEY pela key do config.toml
curl -H "Authorization: Bearer SUA_API_KEY" \
  http://localhost:3000/api/config
```

### **Chat**

```bash
curl -X POST http://localhost:3000/api/chat \
  -H "Authorization: Bearer SUA_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"prompt": "Olá, Jarvis!"}'
```

### **Interface Web**

Abra no navegador: http://localhost:3000

---

## 📊 Estrutura de Arquivos

```
projeto/
├── scripts/
│   ├── test-local-web-api.ps1    # Iniciar servidor (Windows)
│   ├── test-local-web-api.sh     # Iniciar servidor (Linux/Mac)
│   ├── test-api.ps1              # Testar API (Windows)
│   └── test-api.sh                # Testar API (Linux/Mac)
└── jarvis-rs/
    └── target/
        └── release/
            └── jarvis-web-api    # Binário compilado
```

---

## 🐛 Troubleshooting

### **Erro: "Rust não encontrado"**

```bash
# Instalar Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### **Erro: "Porta 3000 já em uso"**

```bash
# Verificar o que está usando a porta
netstat -an | grep 3000  # Linux/Mac
netstat -an | findstr 3000  # Windows

# Parar o processo ou mudar a porta no config.toml
```

### **Erro: "Binário não encontrado"**

```bash
# Recompilar
cd jarvis-rs
cargo build --package jarvis-web-api --release
```

### **Erro: "API key inválida"**

```bash
# Verificar a key no config.toml
cat ~/.jarvis/config.toml  # Linux/Mac
Get-Content "$env:USERPROFILE\.jarvis\config.toml"  # Windows
```

---

## ✅ Checklist

- [ ] Rust instalado (`cargo --version`)
- [ ] Projeto compilado (`cargo build --release`)
- [ ] `config.toml` criado em `~/.jarvis/`
- [ ] API key configurada
- [ ] Servidor rodando (`http://localhost:3000`)
- [ ] Health check passando
- [ ] Interface web acessível

---

## 🎯 Próximos Passos

Após testar localmente:

- ✅ Deploy no Railway: Veja [DEPLOY_RAILWAY.md](./DEPLOY_RAILWAY.md)
- ✅ Deploy com Docker: Veja [DOCKER_COMPOSE.md](./DOCKER_COMPOSE.md)
- ✅ Documentação completa: Veja [README.md](./README.md)
