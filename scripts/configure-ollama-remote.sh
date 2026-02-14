#!/bin/bash
# Configurar Jarvis para usar Ollama remoto via Tailscale
# Execute este script NA MÁQUINA LOCAL

echo "🔧 Configurar Jarvis para Ollama Remoto"
echo "========================================"
echo ""

# Pedir IP Tailscale da VPS
read -p "Digite o IP Tailscale da VPS (ex: 100.x.x.x): " TAILSCALE_IP

if [ -z "$TAILSCALE_IP" ]; then
    echo "❌ IP não pode estar vazio!"
    exit 1
fi

echo ""
echo "🧪 Testando conexão com VPS..."

# Testar conexão
if curl -s --connect-timeout 5 "http://${TAILSCALE_IP}:11434/api/tags" > /dev/null; then
    echo "✅ Conexão com VPS OK!"
else
    echo "❌ Não foi possível conectar à VPS"
    echo "   Verifique:"
    echo "   - Tailscale está ativo na VPS?"
    echo "   - Ollama está rodando na VPS?"
    echo "   - IP está correto?"
    exit 1
fi

echo ""

# Listar modelos disponíveis
echo "📦 Modelos disponíveis na VPS:"
curl -s "http://${TAILSCALE_IP}:11434/api/tags" | jq -r '.models[].name' 2>/dev/null || echo "   (instale jq para ver lista: apt install jq)"
echo ""

# Atualizar config.toml
CONFIG_FILE="$HOME/.jarvis/config.toml"

echo "📝 Atualizando config.toml..."

# Backup do config atual
if [ -f "$CONFIG_FILE" ]; then
    cp "$CONFIG_FILE" "$CONFIG_FILE.backup"
    echo "✅ Backup criado: $CONFIG_FILE.backup"
fi

# Verificar se a seção ollama já existe
if grep -q "\[model_providers.ollama\]" "$CONFIG_FILE" 2>/dev/null; then
    # Atualizar URL existente
    sed -i "s|base_url = \"http://localhost:11434/v1\"|base_url = \"http://${TAILSCALE_IP}:11434/v1\"|g" "$CONFIG_FILE"
    echo "✅ Configuração ollama atualizada"
else
    # Adicionar configuração
    cat >> "$CONFIG_FILE" <<EOF

# Ollama Configuration (Remote via Tailscale)
[model_providers.ollama]
name = "Ollama"
base_url = "http://${TAILSCALE_IP}:11434/v1"
EOF
    echo "✅ Configuração ollama adicionada"
fi

# Definir ollama como provider padrão
if grep -q "^model_provider = " "$CONFIG_FILE" 2>/dev/null; then
    sed -i 's/^model_provider = .*/model_provider = "ollama"/' "$CONFIG_FILE"
else
    sed -i '1i model_provider = "ollama"' "$CONFIG_FILE"
fi

# Definir modelo padrão
if grep -q "^model = " "$CONFIG_FILE" 2>/dev/null; then
    sed -i 's/^model = .*/model = "llama3.2:3b"/' "$CONFIG_FILE"
else
    sed -i '2i model = "llama3.2:3b"' "$CONFIG_FILE"
fi

echo "✅ Provider padrão configurado: ollama"
echo "✅ Modelo padrão configurado: llama3.2:3b"
echo ""

# Criar variável de ambiente
echo "🌐 Criando variável de ambiente..."
export OLLAMA_BASE_URL="http://${TAILSCALE_IP}:11434"
echo "✅ OLLAMA_BASE_URL configurada"
echo ""

# Adicionar ao .bashrc
if ! grep -q "OLLAMA_BASE_URL" ~/.bashrc 2>/dev/null; then
    echo "" >> ~/.bashrc
    echo "# Ollama remoto via Tailscale" >> ~/.bashrc
    echo "export OLLAMA_BASE_URL=\"http://${TAILSCALE_IP}:11434\"" >> ~/.bashrc
    echo "✅ Variável adicionada ao .bashrc"
fi

echo ""
echo "🎉 Configuração concluída!"
echo ""
echo "📋 Resumo:"
echo "   - VPS IP: $TAILSCALE_IP"
echo "   - Porta: 11434"
echo "   - Provider padrão: ollama"
echo "   - Modelo padrão: llama3.2:3b"
echo ""
echo "🧪 Para testar:"
echo "   cd jarvis-rs"
echo "   ./target/debug/jarvis.exe chat"
echo ""
echo "   Ou especificamente:"
echo "   ./target/debug/jarvis.exe chat -c model_provider=ollama -m llama3.2:3b"
echo ""
echo "💡 Modelos disponíveis na VPS:"
echo "   - phi3:mini (ultra-rápido)"
echo "   - llama3.2:3b (recomendado)"
echo "   - llama3.1:8b (mais capaz)"
echo ""
echo "🔄 Para trocar modelo:"
echo "   › /model llama3.1:8b"
echo ""
