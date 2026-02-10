# 🚀 Setup Ollama na VPS via Tailscale

## 🎯 Arquitetura

```
Máquina Local          Tailscale          VPS
┌─────────────┐       (seguro)      ┌─────────────┐
│ Jarvis CLI  │ ────────────────▶   │   Ollama    │
│             │   100.x.x.x:11434   │   Server    │
│ - Interface │                      │ - Modelos   │
│ - Requests  │                      │ - GPU/CPU   │
└─────────────┘                      └─────────────┘
```

## ✅ Vantagens

✅ **Performance** - VPS dedicada para IA
✅ **Recursos** - GPU/CPU otimizado
✅ **Sempre disponível** - 24/7
✅ **Modelos maiores** - Mais RAM disponível
✅ **Seguro** - Acesso via Tailscale
✅ **Multi-dispositivo** - Acesse de qualquer lugar

---

## 📋 Pré-requisitos

- ✅ VPS Linux (Ubuntu/Debian recomendado)
- ✅ Tailscale instalado em ambas máquinas
- ✅ SSH acesso à VPS
- ✅ Mínimo 4GB RAM na VPS (8GB+ recomendado)

---

## 🔧 Passo 1: Setup na VPS

### 1.1 Conectar à VPS via SSH

```bash
ssh user@your-vps-ip
```

### 1.2 Copiar script para VPS

**Na máquina local:**
```bash
scp setup-ollama-vps.sh user@your-vps-ip:/tmp/
```

**Ou criar manualmente na VPS:**
```bash
nano /tmp/setup-ollama-vps.sh
# Cole o conteúdo do script
chmod +x /tmp/setup-ollama-vps.sh
```

### 1.3 Executar script na VPS

```bash
cd /tmp
./setup-ollama-vps.sh
```

Este script irá:
1. ✅ Instalar Ollama
2. ✅ Configurar para aceitar conexões remotas
3. ✅ Baixar modelos (phi3:mini, llama3.2:3b, llama3.1:8b)
4. ✅ Iniciar serviço
5. ✅ Mostrar IP Tailscale

### 1.4 Anotar IP Tailscale

O script mostrará algo como:
```
🌐 IP Tailscale:
100.123.45.67
```

**ANOTE ESTE IP!** Você vai precisar na máquina local.

---

## 🔧 Passo 2: Configurar Máquina Local

### 2.1 Executar script de configuração

**Na máquina local (Windows Git Bash):**

```bash
cd /e/projects/ia/jarvis_cli
chmod +x configure-ollama-remote.sh
./configure-ollama-remote.sh
```

O script vai pedir o IP Tailscale da VPS.

### 2.2 O que o script faz

1. ✅ Testa conexão com VPS
2. ✅ Lista modelos disponíveis
3. ✅ Atualiza `~/.jarvis/config.toml`
4. ✅ Configura variáveis de ambiente
5. ✅ Define ollama como provider padrão

---

## 🧪 Passo 3: Testar

### 3.1 Teste de Conexão

```bash
# Substitua pelo seu IP Tailscale
curl http://100.x.x.x:11434/api/tags
```

Deve retornar JSON com lista de modelos.

### 3.2 Teste com Jarvis

```bash
cd /e/projects/ia/jarvis_cli/jarvis-rs

# Teste simples
./target/debug/jarvis.exe chat

# Teste específico
./target/debug/jarvis.exe chat -c model_provider=ollama -m llama3.2:3b
```

### 3.3 Conversa de Teste

```
› Olá! Você está rodando na VPS via Tailscale?
```

Deve receber resposta do modelo! 🎉

---

## 🔍 Troubleshooting

### Problema: Não conecta à VPS

```bash
# 1. Verificar Tailscale na VPS
tailscale status

# 2. Verificar Ollama na VPS
sudo systemctl status ollama

# 3. Verificar porta 11434
netstat -tuln | grep 11434

# 4. Testar localmente na VPS
curl http://localhost:11434/api/tags

# 5. Verificar firewall (se houver)
sudo ufw status
sudo ufw allow 11434/tcp  # Se necessário
```

### Problema: Ollama não inicia

```bash
# Ver logs
sudo journalctl -u ollama -f

# Reiniciar manualmente
sudo systemctl restart ollama

# Verificar configuração
cat /etc/systemd/system/ollama.service.d/override.conf
```

### Problema: Modelo não carrega

```bash
# Na VPS, verificar espaço em disco
df -h

# Ver modelos instalados
ollama list

# Baixar modelo manualmente
ollama pull llama3.2:3b
```

---

## 📊 Modelos Recomendados por RAM

### VPS com 4GB RAM
```bash
ollama pull phi3:mini         # 256MB
ollama pull llama3.2:3b       # 2GB
```

### VPS com 8GB RAM
```bash
ollama pull llama3.1:8b       # 4.7GB
ollama pull codellama:7b      # 3.8GB
```

### VPS com 16GB+ RAM
```bash
ollama pull llama3.1:70b      # 40GB (requer GPU)
ollama pull mixtral:8x7b      # 26GB
```

---

## 🔐 Segurança

### ✅ Boas Práticas

1. **Usar apenas Tailscale** - Nunca exponha porta 11434 publicamente
2. **Firewall** - Bloquear porta 11434 para internet pública
3. **Monitorar uso** - Verificar logs regularmente
4. **Atualizar** - Manter Ollama e sistema atualizados

### Verificar Configuração de Segurança

```bash
# Na VPS
# 1. Verificar que Ollama só escuta em 0.0.0.0 (não na internet pública)
sudo netstat -tulpn | grep 11434

# 2. Verificar firewall
sudo ufw status

# 3. Se usar ufw, permitir apenas Tailscale
sudo ufw allow in on tailscale0
```

---

## ⚙️ Configuração Avançada

### Usar GPU na VPS (Se Disponível)

```bash
# Instalar drivers NVIDIA (se GPU NVIDIA)
sudo apt install nvidia-driver-535

# Verificar GPU
nvidia-smi

# Ollama detectará GPU automaticamente
# Modelos carregarão na GPU se disponível
```

### Múltiplos Modelos Simultâneos

```bash
# Ollama mantém modelos em memória
# Configure limite de memória em /etc/systemd/system/ollama.service.d/override.conf

[Service]
Environment="OLLAMA_HOST=0.0.0.0:11434"
Environment="OLLAMA_MAX_LOADED_MODELS=3"
Environment="OLLAMA_NUM_PARALLEL=2"
```

### Monitoramento

```bash
# Ver uso de recursos
htop

# Ver logs em tempo real
sudo journalctl -u ollama -f

# Ver modelos carregados
curl http://localhost:11434/api/ps
```

---

## 💡 Dicas

### Otimização de Performance

1. **Use SSD** - Modelos carregam mais rápido
2. **Mais RAM** - Permite modelos maiores
3. **GPU** - Aceleração massiva (10-100x)
4. **Modelos menores** - phi3:mini é ultra-rápido

### Economia de Recursos

```bash
# Usar modelos quantizados (menores, mais rápidos)
ollama pull llama3.2:3b-q4_0  # 4-bit quantization

# Limpar modelos não usados
ollama rm modelo-antigo
```

### Backup

```bash
# Modelos ficam em ~/.ollama/models
# Backup periódico
tar czf ollama-models-backup.tar.gz ~/.ollama/models/
```

---

## 📊 Resumo de Comandos

### Na VPS

```bash
# Setup inicial
./setup-ollama-vps.sh

# Gerenciar modelos
ollama list
ollama pull llama3.2:3b
ollama rm modelo-antigo

# Gerenciar serviço
sudo systemctl status ollama
sudo systemctl restart ollama
sudo journalctl -u ollama -f
```

### Na Máquina Local

```bash
# Configurar
./configure-ollama-remote.sh

# Testar conexão
curl http://100.x.x.x:11434/api/tags

# Usar Jarvis
cd jarvis-rs
./target/debug/jarvis.exe chat

# Trocar modelo
› /model llama3.1:8b
```

---

## 🎉 Resultado Final

```
Máquina Local:
$ ./target/debug/jarvis.exe chat

╭──────────────────────────────────────────────────╮
│ >_ Ollama Jarvis (v0.0.0)                        │
│ model:  llama3.2:3b                              │
╰──────────────────────────────────────────────────╯

› Olá! Você está rodando na VPS?

Sim! Estou rodando via Ollama na VPS através do Tailscale.
Totalmente gratuito e com ótima performance! 🚀
```

---

## 📈 Próximos Passos

1. ✅ Testar diferentes modelos
2. ✅ Ajustar performance conforme necessidade
3. ✅ Adicionar modelos especializados (código, tradução, etc)
4. ✅ Monitorar uso e otimizar
5. ✅ Quando satisfeito, usar para produção!

---

**Custos: $0 (exceto custo da VPS que você já tem)** 🎉
