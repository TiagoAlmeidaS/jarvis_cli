#!/bin/bash
# Script para preparar VPS para Ollama
# Execute este script NA VPS via SSH

echo "🚀 Preparando VPS para Ollama"
echo "=============================="
echo ""

# Atualizar sistema
echo "📦 Atualizando sistema..."
apt update && apt upgrade -y

# Instalar dependências
echo "📦 Instalando dependências..."
apt install -y curl wget git htop netcat-openbsd jq

# Verificar se Tailscale está instalado
if ! command -v tailscale &> /dev/null; then
    echo "📥 Instalando Tailscale..."
    curl -fsSL https://tailscale.com/install.sh | sh
    echo "⚠️  Configure Tailscale: sudo tailscale up"
else
    echo "✅ Tailscale já instalado"
fi

# Mostrar IP Tailscale
echo ""
echo "🌐 IP Tailscale atual:"
tailscale ip -4 2>/dev/null || echo "   Configure com: sudo tailscale up"
echo ""

# Instalar Ollama
echo "📥 Instalando Ollama..."
if ! command -v ollama &> /dev/null; then
    curl -fsSL https://ollama.com/install.sh | sh
    echo "✅ Ollama instalado"
else
    echo "✅ Ollama já instalado"
fi

# Configurar Ollama para aceitar conexões remotas
echo ""
echo "🔧 Configurando Ollama para conexões remotas..."

mkdir -p /etc/systemd/system/ollama.service.d/

cat > /etc/systemd/system/ollama.service.d/override.conf <<'EOF'
[Service]
Environment="OLLAMA_HOST=0.0.0.0:11434"
Environment="OLLAMA_NUM_PARALLEL=2"
Environment="OLLAMA_MAX_LOADED_MODELS=3"
EOF

echo "✅ Configuração criada"

# Recarregar e reiniciar
echo ""
echo "🔄 Reiniciando Ollama..."
systemctl daemon-reload
systemctl enable ollama
systemctl restart ollama

sleep 3

# Verificar status
echo ""
echo "📊 Status do Ollama:"
systemctl status ollama --no-pager | head -15

# Verificar porta
echo ""
echo "🔍 Verificando porta 11434..."
if netstat -tuln | grep -q ":11434"; then
    echo "✅ Porta 11434 aberta e escutando"
else
    echo "⚠️  Porta 11434 não encontrada"
fi

# Baixar modelos recomendados
echo ""
echo "📦 Baixando modelos..."
echo ""

# Verificar RAM disponível
TOTAL_RAM=$(free -g | awk '/^Mem:/{print $2}')
echo "💾 RAM disponível: ${TOTAL_RAM}GB"
echo ""

# Baixar modelos baseado na RAM
if [ "$TOTAL_RAM" -ge 16 ]; then
    echo "🚀 RAM suficiente para modelos grandes!"
    echo ""
    ollama pull phi3:mini
    ollama pull llama3.2:3b
    ollama pull llama3.1:8b
    ollama pull codellama:7b
elif [ "$TOTAL_RAM" -ge 8 ]; then
    echo "⚡ RAM suficiente para modelos médios"
    echo ""
    ollama pull phi3:mini
    ollama pull llama3.2:3b
    ollama pull llama3.1:8b
else
    echo "💡 RAM limitada - usando modelos leves"
    echo ""
    ollama pull phi3:mini
    ollama pull llama3.2:3b
fi

# Listar modelos instalados
echo ""
echo "📦 Modelos instalados:"
ollama list

# Testar servidor
echo ""
echo "🧪 Testando servidor..."
if curl -s http://localhost:11434/api/tags > /dev/null; then
    echo "✅ Servidor respondendo!"
else
    echo "❌ Erro ao conectar"
fi

# Configurar firewall básico (se ufw estiver disponível)
if command -v ufw &> /dev/null; then
    echo ""
    echo "🔐 Configurando firewall..."
    ufw allow 22/tcp
    ufw allow in on tailscale0
    echo "✅ Firewall configurado"
fi

# Informações finais
echo ""
echo "🎉 Setup concluído!"
echo ""
echo "📋 Informações importantes:"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

# IP Tailscale
TAILSCALE_IP=$(tailscale ip -4 2>/dev/null)
if [ ! -z "$TAILSCALE_IP" ]; then
    echo "🌐 IP Tailscale: $TAILSCALE_IP"
else
    echo "⚠️  Configure Tailscale: sudo tailscale up"
fi

echo "🔌 Porta: 11434"
echo "💾 RAM: ${TOTAL_RAM}GB"
echo ""
echo "📦 Modelos:"
ollama list | tail -n +2 | awk '{print "   - " $1}'
echo ""
echo "🧪 Teste local:"
echo "   curl http://localhost:11434/api/tags"
echo ""
echo "🔐 Segurança:"
echo "   ✅ Acesso via Tailscale (seguro)"
echo "   ⚠️  NÃO exponha porta 11434 publicamente"
echo ""
echo "📝 Próximo passo:"
echo "   Configure o Jarvis na máquina local"
echo "   Use o IP Tailscale: $TAILSCALE_IP"
echo ""
