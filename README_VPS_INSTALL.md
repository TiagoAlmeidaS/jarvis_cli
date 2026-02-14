# 🚀 Instalação VPS - Um Comando

## 📋 Pré-requisitos

- ✅ VPS com Ubuntu/Debian (16.04+)
- ✅ Acesso SSH como root
- ✅ Mínimo 4GB RAM (8GB+ recomendado)
- ✅ 20GB+ espaço em disco

---

## ⚡ Instalação Rápida (3 Passos)

### 1️⃣ Clonar Projeto na VPS

```bash
# Conectar à VPS
ssh root@76.13.96.99

# Clonar projeto
cd /opt
git clone <url-do-projeto> jarvis
cd jarvis

# OU se já baixou localmente, envie via scp:
# scp -r /e/projects/ia/jarvis_cli root@76.13.96.99:/opt/jarvis
```

### 2️⃣ Executar Script de Instalação

```bash
# Na VPS
cd /opt/jarvis
chmod +x install-vps.sh
sudo ./install-vps.sh
```

O script irá:
1. ✅ Instalar dependências
2. ✅ Instalar Docker
3. ✅ Instalar Tailscale
4. ✅ Perguntar método (Docker ou Nativo)
5. ✅ Instalar Ollama
6. ✅ Baixar modelos (baseado na RAM)
7. ✅ Configurar firewall
8. ✅ Criar scripts de manutenção
9. ✅ Testar instalação

### 3️⃣ Configurar Máquina Local

```bash
# Na máquina local
cd /e/projects/ia/jarvis_cli
./scripts/configure-ollama-remote.sh
# Digite o IP Tailscale quando solicitado
```

---

## 🎯 Uso

### Testar

```bash
cd jarvis-rs
./target/debug/jarvis.exe chat

› Olá! Você está funcionando?
```

### Comandos na VPS

```bash
# Ver status
ollama-status

# Ver logs
ollama-logs

# Reiniciar
ollama-restart

# Listar modelos (Docker)
docker exec ollama ollama list

# Listar modelos (Nativo)
ollama list

# Baixar novo modelo
docker exec ollama ollama pull llama3.1:8b  # Docker
ollama pull llama3.1:8b                      # Nativo
```

---

## 🐳 Docker vs Nativo

### Docker (Recomendado)

**Vantagens:**
- ✅ Isolado do sistema
- ✅ Fácil atualização
- ✅ Fácil backup
- ✅ Suporte a GPU automático

**Desvantagens:**
- ⚠️ Leve overhead

### Nativo

**Vantagens:**
- ✅ Melhor performance
- ✅ Menor latência

**Desvantagens:**
- ⚠️ Mais difícil de gerenciar

---

## 📊 Modelos por RAM

### 4GB RAM
```bash
- phi3:mini (256MB)
- llama3.2:3b (2GB)
```

### 8GB RAM
```bash
- phi3:mini (256MB)
- llama3.2:3b (2GB)
- llama3.1:8b (4.7GB)
```

### 16GB+ RAM
```bash
- Todos acima +
- codellama:7b (3.8GB)
- mixtral:8x7b (26GB)
- llama3.1:70b (40GB, requer GPU)
```

---

## 🔍 Troubleshooting

### Ollama não inicia

```bash
# Ver logs
ollama-logs

# Reiniciar
ollama-restart

# Verificar status
ollama-status
```

### Não conecta da máquina local

```bash
# Na VPS: verificar porta
netstat -tuln | grep 11434

# Na VPS: verificar Tailscale
tailscale status

# Na máquina local: testar conexão
curl http://[IP_TAILSCALE]:11434/api/tags
```

### Modelo não carrega

```bash
# Verificar espaço
df -h

# Verificar RAM
free -h

# Baixar modelo menor
docker exec ollama ollama pull phi3:mini  # Docker
ollama pull phi3:mini                      # Nativo
```

---

## 🔐 Segurança

### Configuração Automática

O script configura:
- ✅ Porta 11434 apenas via Tailscale
- ✅ Firewall UFW (se disponível)
- ✅ SSH na porta 22

### Verificar

```bash
# Status do firewall
sudo ufw status

# Portas abertas
netstat -tuln

# Tailscale
tailscale status
```

---

## 🔄 Atualização

### Docker

```bash
docker pull ollama/ollama:latest
docker restart ollama
```

### Nativo

```bash
curl -fsSL https://ollama.com/install.sh | sh
systemctl restart ollama
```

---

## 💾 Backup

### Docker

```bash
# Backup dos modelos
tar czf ollama-backup.tar.gz /opt/ollama/data

# Restaurar
tar xzf ollama-backup.tar.gz -C /
```

### Nativo

```bash
# Backup
tar czf ollama-backup.tar.gz ~/.ollama

# Restaurar
tar xzf ollama-backup.tar.gz -C ~/
```

---

## 📈 Monitoramento

### Recursos

```bash
# CPU e RAM
htop

# Espaço
df -h

# Modelos carregados
curl http://localhost:11434/api/ps
```

### Logs em Tempo Real

```bash
ollama-logs
```

---

## 🎉 Resultado Final

```
VPS:
✓ Ollama rodando (Docker ou Nativo)
✓ Porta 11434 acessível via Tailscale
✓ Modelos: phi3:mini, llama3.2:3b, llama3.1:8b
✓ Scripts de manutenção instalados

Máquina Local:
✓ Jarvis configurado para usar VPS
✓ Zero custos de tokens!
✓ Performance dedicada!
```

---

## 📞 Suporte

- **Documentação completa:** `OLLAMA_VPS_SETUP.md`
- **Informações da instalação:** `VPS_INFO.txt` (criado após instalação)
- **Comandos úteis:** `ollama-status`, `ollama-logs`, `ollama-restart`

---

## 💰 Custos

| Item | Custo |
|------|-------|
| VPS | Já tem ✅ |
| Ollama | $0 |
| Modelos | $0 |
| Tailscale | $0 |
| **TOTAL** | **$0** 🎉 |

---

**Execute agora e comece a usar!** 🚀
