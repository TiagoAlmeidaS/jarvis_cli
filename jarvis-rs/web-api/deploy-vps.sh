#!/bin/bash
# Script de Deploy para VPS - Linux/Mac
# Facilita o processo de build e transferência

set -e

VPS_USER="${1:-}"
VPS_IP="${2:-}"
VPS_PATH="${3:-/opt/jarvis}"

if [ -z "$VPS_USER" ] || [ -z "$VPS_IP" ]; then
    echo "Uso: $0 <usuario> <ip-vps> [caminho-vps]"
    echo "Exemplo: $0 ubuntu 192.168.1.100 /opt/jarvis"
    exit 1
fi

echo "🚀 Deploy Jarvis Web API para VPS"
echo "===================================="

# 1. Verificar pré-requisitos
echo ""
echo "📋 Verificando pré-requisitos..."

if ! command -v rustc &> /dev/null; then
    echo "❌ Rust não encontrado! Instale em: https://rustup.rs"
    exit 1
fi
echo "✅ Rust encontrado: $(rustc --version)"

# Verificar target Linux
echo ""
echo "🔧 Verificando target Linux..."
if ! rustup target list --installed | grep -q "x86_64-unknown-linux-musl"; then
    echo "📦 Instalando target Linux..."
    rustup target add x86_64-unknown-linux-musl
fi
echo "✅ Target Linux instalado"

# 2. Compilar
echo ""
echo "🔨 Compilando para Linux..."
cd ..
cargo build --package jarvis-web-api --release --target x86_64-unknown-linux-musl
echo "✅ Compilação concluída!"
cd web-api

# 3. Verificar binário
BINARY_PATH="../target/x86_64-unknown-linux-musl/release/jarvis-web-api"
if [ ! -f "$BINARY_PATH" ]; then
    echo "❌ Binário não encontrado em: $BINARY_PATH"
    exit 1
fi
echo "✅ Binário encontrado"

# 4. Gerar API Key (se não existir)
echo ""
echo "🔑 Verificando API Key..."
API_KEY_FILE="$HOME/.jarvis-api-key.txt"
if [ ! -f "$API_KEY_FILE" ]; then
    API_KEY=$(openssl rand -hex 32)
    echo "$API_KEY" > "$API_KEY_FILE"
    chmod 600 "$API_KEY_FILE"
    echo "✅ Nova API Key gerada: $API_KEY"
    echo "⚠️  Salve esta key em local seguro!"
else
    API_KEY=$(cat "$API_KEY_FILE")
    echo "✅ API Key existente encontrada"
fi

# 5. Transferir binário
echo ""
echo "📤 Transferindo binário para VPS..."
scp "$BINARY_PATH" "${VPS_USER}@${VPS_IP}:${VPS_PATH}/jarvis-web-api"
echo "✅ Binário transferido!"

# 6. Transferir arquivos estáticos
echo ""
echo "📤 Transferindo arquivos estáticos..."
if [ -d "static" ]; then
    scp -r static "${VPS_USER}@${VPS_IP}:${VPS_PATH}/static" || echo "⚠️  Aviso: Erro ao transferir estáticos"
    echo "✅ Arquivos estáticos transferidos!"
fi

# 7. Instruções finais
echo ""
echo "✅ Deploy concluído!"
echo ""
echo "📋 Próximos passos na VPS:"
echo "1. Conectar: ssh ${VPS_USER}@${VPS_IP}"
echo "2. Tornar executável: chmod +x ${VPS_PATH}/jarvis-web-api"
echo "3. Criar config.toml em ${VPS_PATH}/.jarvis/config.toml"
echo "   Com API Key: $API_KEY"
echo "4. Criar systemd service (veja DEPLOY_VPS.md)"
echo "5. Iniciar: sudo systemctl start jarvis-web-api"
echo ""
echo "📚 Veja DEPLOY_VPS.md para instruções completas"
