# 🚀 Guia de Execução na VPS

## ⚠️ ATENÇÃO: Execute Você Mesmo

**Por segurança, você deve executar estes comandos manualmente via SSH.**

---

## 📋 Passo 1: Conectar à VPS

```bash
# No Git Bash da sua máquina local
ssh root@76.13.96.99
# Digite a senha quando solicitado
```

---

## 📋 Passo 2: Copiar Script para VPS

**Opção A: Via SCP (Recomendado)**

```bash
# Na máquina local (outro terminal)
cd /e/projects/ia/jarvis_cli
scp scripts/deploy-ollama-vps.sh root@76.13.96.99:/tmp/
```

**Opção B: Criar Manualmente na VPS**

```bash
# Já conectado na VPS
nano /tmp/deploy-ollama-vps.sh
# Cole o conteúdo do arquivo scripts/deploy-ollama-vps.sh
# Ctrl+X, Y, Enter para salvar
```

---

## 📋 Passo 3: Executar Script na VPS

```bash
# Na VPS
cd /tmp
chmod +x deploy-ollama-vps.sh
./deploy-ollama-vps.sh
```

**O script irá:**
1. ✅ Atualizar sistema
2. ✅ Instalar Tailscale (se necessário)
3. ✅ Instalar Ollama
4. ✅ Configurar para aceitar conexões remotas
5. ✅ Baixar modelos (baseado na RAM disponível)
6. ✅ Mostrar IP Tailscale

**IMPORTANTE:** Anote o IP Tailscale mostrado no final!

---

## 📋 Passo 4: Verificar Tailscale

Se Tailscale não estiver configurado:

```bash
# Na VPS
sudo tailscale up
# Siga as instruções no terminal
```

Verificar IP:
```bash
tailscale ip -4
```

---

## 📋 Passo 5: Testar na VPS

```bash
# Na VPS
curl http://localhost:11434/api/tags

# Deve retornar JSON com modelos
```

---

## 📋 Passo 6: Configurar Máquina Local

**Na máquina local (Git Bash):**

```bash
cd /e/projects/ia/jarvis_cli

# Executar script de configuração
./scripts/configure-ollama-remote.sh
# Digite o IP Tailscale quando solicitado
```

---

## 📋 Passo 7: Testar Jarvis

```bash
cd jarvis-rs
./target/debug/jarvis.exe chat

› Olá! Você está funcionando via VPS?
```

---

## 🔍 Verificação Rápida

### Na VPS

```bash
# Status do serviço
systemctl status ollama

# Ver logs
journalctl -u ollama -f

# Listar modelos
ollama list

# Testar
curl http://localhost:11434/api/tags
```

### Na Máquina Local

```bash
# Testar conexão (substitua pelo seu IP Tailscale)
curl http://100.x.x.x:11434/api/tags

# Testar Jarvis
cd jarvis-rs
./target/debug/jarvis.exe chat -c model_provider=ollama
```

---

## 🛠️ Comandos Úteis na VPS

### Gerenciar Ollama

```bash
# Reiniciar
systemctl restart ollama

# Ver logs
journalctl -u ollama -n 50

# Adicionar modelo
ollama pull llama3.1:8b

# Remover modelo
ollama rm modelo-antigo

# Ver modelos carregados
curl http://localhost:11434/api/ps
```

### Verificar Recursos

```bash
# Uso de RAM
free -h

# Uso de CPU
htop

# Espaço em disco
df -h

# Processos Ollama
ps aux | grep ollama
```

### Monitoramento

```bash
# Ver uso em tempo real
watch -n 1 'free -h && echo && ollama list'
```

---

## 🔐 Segurança

### Verificar Configuração

```bash
# Na VPS

# 1. Porta 11434 deve estar escutando
netstat -tuln | grep 11434

# 2. Tailscale deve estar ativo
tailscale status

# 3. Firewall (se usar ufw)
ufw status
```

### Recomendações

1. ✅ Usar apenas Tailscale (já configurado)
2. ✅ NÃO expor porta 11434 publicamente
3. ✅ Manter sistema atualizado
4. ✅ Monitorar logs regularmente

---

## 📊 Modelos por RAM

### 4GB RAM (Mínimo)
- phi3:mini (256MB)
- llama3.2:3b (2GB)

### 8GB RAM (Bom)
- phi3:mini (256MB)
- llama3.2:3b (2GB)
- llama3.1:8b (4.7GB)

### 16GB+ RAM (Ótimo)
- Todos acima +
- codellama:7b (3.8GB)
- mixtral:8x7b (26GB)

---

## 🎯 Resumo

| Etapa | Comando | Local |
|-------|---------|-------|
| Conectar | `ssh root@76.13.96.99` | Local |
| Copiar script | `scp scripts/deploy-ollama-vps.sh root@76.13.96.99:/tmp/` | Local |
| Executar | `./deploy-ollama-vps.sh` | VPS |
| Configurar Tailscale | `tailscale up` | VPS |
| Configurar local | `./scripts/configure-ollama-remote.sh` | Local |
| Testar | `./target/debug/jarvis.exe chat` | Local |

---

## 💡 Dicas

1. **Mantenha sessão SSH aberta** durante setup
2. **Anote IP Tailscale** imediatamente
3. **Teste cada passo** antes de continuar
4. **Verifique logs** se algo falhar

---

## 🆘 Se Algo Der Errado

```bash
# Verificar tudo
systemctl status ollama
journalctl -u ollama -n 100
tailscale status
netstat -tuln | grep 11434
curl http://localhost:11434/api/tags
```

---

**Execute estes comandos e me avise se precisar de ajuda!** 🚀
