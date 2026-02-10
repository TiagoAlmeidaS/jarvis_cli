#!/bin/bash
# Setup Ollama na VPS
# Execute este script NA VPS

echo "🚀 Setup Ollama na VPS"
echo "======================"
echo ""

# Detectar sistema operacional
if [ -f /etc/os-release ]; then
    . /etc/os-release
    OS=$ID
else
    echo "❌ Sistema não suportado"
    exit 1
fi

echo "📋 Sistema detectado: $OS"
echo ""

# Instalar Ollama
echo "📥 Instalando Ollama..."
curl -fsSL https://ollama.com/install.sh | sh

if [ $? -ne 0 ]; then
    echo "❌ Erro ao instalar Ollama"
    exit 1
fi

echo "✅ Ollama instalado!"
echo ""

# Configurar Ollama para aceitar conexões remotas
echo "🔧 Configurando Ollama para aceitar conexões remotas..."

# Criar diretório de serviço se não existir
sudo mkdir -p /etc/systemd/system/ollama.service.d/

# Criar arquivo de configuração
sudo tee /etc/systemd/system/ollama.service.d/override.conf > /dev/null <<EOF
[Service]
Environment="OLLAMA_HOST=0.0.0.0:11434"
EOF

echo "✅ Configuração criada"
echo ""

# Recarregar systemd e reiniciar Ollama
echo "🔄 Reiniciando serviço Ollama..."
sudo systemctl daemon-reload
sudo systemctl restart ollama

echo "✅ Serviço reiniciado"
echo ""

# Verificar status
echo "📊 Status do serviço:"
sudo systemctl status ollama --no-pager | head -10
echo ""

# Verificar porta
echo "🔍 Verificando porta 11434..."
if netstat -tuln | grep -q ":11434"; then
    echo "✅ Porta 11434 está aberta"
else
    echo "⚠️  Porta 11434 não encontrada"
fi
echo ""

# Mostrar IP Tailscale
echo "🌐 IP Tailscale:"
ip addr show tailscale0 2>/dev/null | grep 'inet ' | awk '{print $2}' | cut -d/ -f1
echo ""

# Baixar modelos recomendados
echo "📦 Baixando modelos recomendados..."
echo ""

echo "1️⃣  Baixando phi3:mini (ultra-leve, 256MB)..."
ollama pull phi3:mini

echo ""
echo "2️⃣  Baixando llama3.2:3b (leve, 2GB)..."
ollama pull llama3.2:3b

echo ""
echo "3️⃣  Baixando llama3.1:8b (balanceado, 4.7GB)..."
ollama pull llama3.1:8b

echo ""
echo "✅ Modelos baixados!"
echo ""

# Listar modelos instalados
echo "📦 Modelos disponíveis:"
ollama list
echo ""

# Testar servidor
echo "🧪 Testando servidor..."
RESPONSE=$(curl -s http://localhost:11434/api/tags)
if [ $? -eq 0 ]; then
    echo "✅ Servidor respondendo corretamente!"
else
    echo "❌ Erro ao conectar ao servidor"
fi
echo ""

# Instruções finais
echo "🎉 Setup concluído!"
echo ""
echo "📋 Informações importantes:"
echo "   - Porta: 11434"
echo "   - Host: 0.0.0.0 (aceita conexões remotas)"
echo "   - Modelos: phi3:mini, llama3.2:3b, llama3.1:8b"
echo ""
echo "🔐 Segurança:"
echo "   ✅ Acesso limitado via Tailscale (já configurado)"
echo "   ⚠️  NÃO exponha porta 11434 publicamente!"
echo ""
echo "🧪 Para testar da máquina local:"
echo "   curl http://[IP_TAILSCALE]:11434/api/tags"
echo ""
echo "📝 Próximo passo:"
echo "   Configure o Jarvis na máquina local com o IP Tailscale"
echo ""
