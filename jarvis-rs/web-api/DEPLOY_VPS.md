# 🖥️ Deploy na VPS - Guia Completo

Deploy na sua própria VPS oferece:
- ✅ **Controle total** sobre o ambiente
- ✅ **Mais seguro** - dados ficam na sua infraestrutura
- ✅ **Sem limitações** de serviços gratuitos
- ✅ **Isolamento completo** para testes
- ✅ **Performance previsível**

---

## 📋 Pré-requisitos

1. **VPS com:**
   - Linux (Ubuntu/Debian recomendado)
   - Acesso SSH
   - Pelo menos 1GB RAM
   - 10GB+ espaço em disco

2. **No seu PC:**
   - Rust instalado
   - Acesso SSH à VPS
   - `scp` ou `rsync` para transferir arquivos

---

## 🚀 Passo a Passo

### **Fase 1: Preparar Build Local**

#### 1.1. Compilar para Linux

```powershell
# No Windows, no diretório jarvis-rs
cd E:\projects\ia\jarvis_cli\jarvis-rs

# Instalar target Linux (se ainda não tiver)
rustup target add x86_64-unknown-linux-musl

# Compilar
cargo build --package jarvis-web-api --release --target x86_64-unknown-linux-musl
```

O binário estará em: `target/x86_64-unknown-linux-musl/release/jarvis-web-api`

#### 1.2. Gerar API Key

```powershell
# Windows PowerShell
$apiKey = -join ((65..90) + (97..122) + (48..57) | Get-Random -Count 32 | ForEach-Object {[char]$_})
Write-Host "API Key: $apiKey"
```

**⚠️ Salve esta API key em local seguro!**

---

### **Fase 2: Configurar VPS**

#### 2.1. Conectar na VPS

```bash
ssh usuario@seu-vps-ip
```

#### 2.2. Criar estrutura de diretórios

```bash
# Criar diretório para Jarvis
sudo mkdir -p /opt/jarvis
sudo mkdir -p /opt/jarvis/.jarvis
sudo chown -R $USER:$USER /opt/jarvis

# Criar diretório para logs
sudo mkdir -p /var/log/jarvis
sudo chown $USER:$USER /var/log/jarvis
```

#### 2.3. Criar config.toml

```bash
nano /opt/jarvis/.jarvis/config.toml
```

Cole o seguinte (substitua `SUA_API_KEY_AQUI`):

```toml
[api]
api_key = "SUA_API_KEY_AQUI"
port = 3000
bind_address = "0.0.0.0"
enable_cors = false  # false em produção, true apenas para testes
```

Salve: `Ctrl+O`, `Enter`, `Ctrl+X`

---

### **Fase 3: Transferir Binário**

#### 3.1. Do seu PC (Windows PowerShell)

```powershell
# Ajuste os valores:
$VPS_USER = "seu-usuario"
$VPS_IP = "seu-ip-vps"
$BINARY_PATH = "E:\projects\ia\jarvis_cli\jarvis-rs\target\x86_64-unknown-linux-musl\release\jarvis-web-api"

# Transferir binário
scp $BINARY_PATH ${VPS_USER}@${VPS_IP}:/opt/jarvis/jarvis-web-api

# Transferir arquivos estáticos (se necessário)
scp -r E:\projects\ia\jarvis_cli\jarvis-rs\web-api\static ${VPS_USER}@${VPS_IP}:/opt/jarvis/static
```

#### 3.2. Na VPS: Tornar executável

```bash
chmod +x /opt/jarvis/jarvis-web-api
```

---

### **Fase 4: Configurar Systemd Service**

#### 4.1. Criar service file

```bash
sudo nano /etc/systemd/system/jarvis-web-api.service
```

Cole o seguinte:

```ini
[Unit]
Description=Jarvis Web API Server
After=network.target

[Service]
Type=simple
User=seu-usuario
WorkingDirectory=/opt/jarvis
Environment="JARVIS_HOME=/opt/jarvis/.jarvis"
Environment="RUST_LOG=info"
ExecStart=/opt/jarvis/jarvis-web-api
Restart=always
RestartSec=10
StandardOutput=journal
StandardError=journal
SyslogIdentifier=jarvis-web-api

# Limites de segurança
LimitNOFILE=65536
MemoryMax=2G

[Install]
WantedBy=multi-user.target
```

**⚠️ Substitua `seu-usuario` pelo seu usuário real!**

Salve: `Ctrl+O`, `Enter`, `Ctrl+X`

#### 4.2. Ativar e iniciar serviço

```bash
# Recarregar systemd
sudo systemctl daemon-reload

# Habilitar para iniciar no boot
sudo systemctl enable jarvis-web-api

# Iniciar serviço
sudo systemctl start jarvis-web-api

# Verificar status
sudo systemctl status jarvis-web-api

# Ver logs
sudo journalctl -u jarvis-web-api -f
```

---

### **Fase 5: Configurar Firewall**

#### 5.1. UFW (Ubuntu/Debian)

```bash
# Permitir SSH (importante!)
sudo ufw allow 22/tcp

# Permitir porta da API
sudo ufw allow 3000/tcp

# Ou, se usar Nginx, permitir apenas 80 e 443
sudo ufw allow 80/tcp
sudo ufw allow 443/tcp

# Ativar firewall
sudo ufw enable

# Verificar status
sudo ufw status
```

---

### **Fase 6: Testar**

#### 6.1. Testar localmente na VPS

```bash
# Health check
curl http://localhost:3000/api/health

# Deve retornar: {"status":"ok","version":"0.0.0"}
```

#### 6.2. Testar remotamente (do seu PC)

```powershell
# Health check
curl http://seu-ip-vps:3000/api/health

# Chat (substitua SUA_API_KEY)
$headers = @{
    "Authorization" = "Bearer SUA_API_KEY"
    "Content-Type" = "application/json"
}
$body = @{
    prompt = "Olá, Jarvis!"
} | ConvertTo-Json

Invoke-RestMethod -Uri "http://seu-ip-vps:3000/api/chat" -Method Post -Headers $headers -Body $body
```

---

## 🔒 Configuração Opcional: Nginx (Recomendado)

### Por que usar Nginx?
- ✅ HTTPS automático (Let's Encrypt)
- ✅ Proxy reverso (mais seguro)
- ✅ Rate limiting
- ✅ Compressão
- ✅ Cache de arquivos estáticos

### Instalar Nginx

```bash
sudo apt update
sudo apt install nginx certbot python3-certbot-nginx -y
```

### Configurar Nginx

```bash
sudo nano /etc/nginx/sites-available/jarvis-api
```

Cole:

```nginx
server {
    listen 80;
    server_name seu-dominio.com;  # Ou IP da VPS

    # Redirecionar para HTTPS (após configurar SSL)
    # return 301 https://$server_name$request_uri;

    # Ou, para testes sem SSL:
    location / {
        proxy_pass http://localhost:3000;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection 'upgrade';
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        proxy_cache_bypass $http_upgrade;
        
        # Timeouts
        proxy_connect_timeout 60s;
        proxy_send_timeout 60s;
        proxy_read_timeout 60s;
    }
}
```

Ativar:

```bash
sudo ln -s /etc/nginx/sites-available/jarvis-api /etc/nginx/sites-enabled/
sudo nginx -t  # Testar configuração
sudo systemctl reload nginx
```

### Configurar HTTPS (Opcional mas Recomendado)

```bash
# Se tiver domínio
sudo certbot --nginx -d seu-dominio.com

# Seguir instruções do certbot
# HTTPS será configurado automaticamente
```

---

## 📊 Comandos Úteis

### Gerenciar Serviço

```bash
# Ver status
sudo systemctl status jarvis-web-api

# Ver logs em tempo real
sudo journalctl -u jarvis-web-api -f

# Ver últimas 100 linhas
sudo journalctl -u jarvis-web-api -n 100

# Reiniciar
sudo systemctl restart jarvis-web-api

# Parar
sudo systemctl stop jarvis-web-api

# Iniciar
sudo systemctl start jarvis-web-api
```

### Atualizar Binário

```bash
# 1. Parar serviço
sudo systemctl stop jarvis-web-api

# 2. Fazer backup
sudo cp /opt/jarvis/jarvis-web-api /opt/jarvis/jarvis-web-api.backup

# 3. Transferir novo binário (do seu PC)
# scp novo-binario usuario@vps:/opt/jarvis/jarvis-web-api

# 4. Tornar executável
chmod +x /opt/jarvis/jarvis-web-api

# 5. Iniciar serviço
sudo systemctl start jarvis-web-api

# 6. Verificar
sudo systemctl status jarvis-web-api
```

---

## 🔍 Troubleshooting

### Serviço não inicia

```bash
# Ver logs detalhados
sudo journalctl -u jarvis-web-api -n 50 --no-pager

# Verificar permissões
ls -la /opt/jarvis/jarvis-web-api

# Verificar config.toml
cat /opt/jarvis/.jarvis/config.toml

# Testar manualmente
/opt/jarvis/jarvis-web-api
```

### Porta já em uso

```bash
# Verificar o que está usando a porta 3000
sudo lsof -i :3000
# ou
sudo netstat -tulpn | grep 3000

# Matar processo se necessário
sudo kill -9 <PID>
```

### Erro de permissão

```bash
# Verificar ownership
ls -la /opt/jarvis

# Corrigir se necessário
sudo chown -R $USER:$USER /opt/jarvis
```

### Firewall bloqueando

```bash
# Verificar UFW
sudo ufw status verbose

# Verificar iptables (se usar)
sudo iptables -L -n -v
```

---

## ✅ Checklist de Deploy

- [ ] Binário compilado para Linux
- [ ] API Key gerada e salva
- [ ] Estrutura de diretórios criada na VPS
- [ ] config.toml criado com API key
- [ ] Binário transferido e executável
- [ ] Systemd service criado e ativado
- [ ] Firewall configurado
- [ ] Serviço rodando (`systemctl status`)
- [ ] Health check funcionando localmente
- [ ] Health check funcionando remotamente
- [ ] Nginx configurado (opcional)
- [ ] HTTPS configurado (opcional)

---

## 🎯 Próximos Passos Após Deploy

1. **Testar acesso remoto**
   - Do celular
   - De outro computador
   - Interface web funcionando

2. **Configurar domínio** (opcional)
   - Apontar DNS para IP da VPS
   - Configurar HTTPS com Let's Encrypt

3. **Monitoramento básico**
   - Verificar logs regularmente
   - Configurar alertas (opcional)

4. **Backup**
   - Fazer backup do `config.toml`
   - Backup do diretório `.jarvis` (se necessário)

---

## 📚 Referências

- [Documentação da API](./README.md)
- [Guia de Deploy Completo](./DEPLOY.md)
- [Status do Plano](./PLANO_STATUS.md)
