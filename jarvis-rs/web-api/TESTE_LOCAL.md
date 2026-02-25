# 🧪 Teste Local - Guia Rápido

## ⚡ Testar em 3 Passos (Sem Docker!)

### **1. Compilar**

```bash
cd jarvis-rs
cargo build --package jarvis-web-api --release
```

### **2. Configurar (Primeira Vez)**

```bash
# Linux/Mac
mkdir -p ~/.jarvis
cat > ~/.jarvis/config.toml << EOF
[api]
api_key = "$(openssl rand -hex 32)"
port = 3000
bind_address = "127.0.0.1"
enable_cors = true
EOF

# Windows PowerShell
New-Item -ItemType Directory -Force -Path "$env:USERPROFILE\.jarvis" | Out-Null
$apiKey = -join ((65..90) + (97..122) + (48..57) | Get-Random -Count 32 | ForEach-Object {[char]$_})
@"
[api]
api_key = "$apiKey"
port = 3000
bind_address = "127.0.0.1"
enable_cors = true
"@ | Out-File -FilePath "$env:USERPROFILE\.jarvis\config.toml" -Encoding UTF8
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
# Abra no navegador: http://localhost:3000
```

---

## 🐳 Com Docker Compose (Opcional)

Se preferir isolar o ambiente:

### **1. Criar docker-compose.yml**

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

volumes:
  jarvis-data:
```

### **2. Rodar**

```bash
# Gerar API key
export JARVIS_API_KEY=$(openssl rand -hex 32)

# Subir
docker-compose up -d

# Ver logs
docker-compose logs -f jarvis-web-api

# Parar
docker-compose down
```

---

## ✅ Pronto!

Agora você pode:
- Acessar interface web: http://localhost:3000
- Testar API: `curl http://localhost:3000/api/health`
- Ver logs no terminal

**Não precisa de banco de dados separado!** Tudo funciona automaticamente. 🎉
