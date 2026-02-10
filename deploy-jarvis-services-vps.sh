#!/bin/bash
# Script para fazer deploy dos serviços Jarvis na VPS via Tailscale

set -e

# Configurações
VPS_IP="100.98.213.86"  # IP do Tailscale da VPS
VPS_USER="root"         # Usuário SSH
VPS_DIR="/opt/jarvis"   # Diretório na VPS

echo "🚀 Deploy dos Serviços Jarvis na VPS"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "VPS: $VPS_USER@$VPS_IP"
echo "Diretório: $VPS_DIR"
echo ""

# Verificar conectividade
echo "🔍 Verificando conectividade com VPS..."
if ! ping -c 1 $VPS_IP > /dev/null 2>&1; then
    echo "❌ Não foi possível conectar ao VPS via Tailscale"
    echo "   Verifique se o Tailscale está ativo e a VPS está online"
    exit 1
fi
echo "✅ VPS acessível via Tailscale"
echo ""

# Criar diretório na VPS
echo "📁 Criando diretório na VPS..."
ssh $VPS_USER@$VPS_IP "mkdir -p $VPS_DIR"

# Copiar docker-compose
echo "📦 Copiando docker-compose.vps.yml..."
scp docker-compose.vps.yml $VPS_USER@$VPS_IP:$VPS_DIR/docker-compose.yml

# Criar .env na VPS
echo "⚙️  Criando arquivo .env..."
ssh $VPS_USER@$VPS_IP "cat > $VPS_DIR/.env" << 'EOF'
# PostgreSQL
POSTGRES_PASSWORD=jarvis_secure_password_2026
POSTGRES_DB=jarvis
POSTGRES_USER=jarvis

# Qdrant (sem senha por padrão, protegido pela rede Tailscale)
QDRANT_API_KEY=

# Redis (sem senha por padrão, protegido pela rede Tailscale)
REDIS_PASSWORD=
EOF

echo "✅ Arquivo .env criado"
echo ""

# Fazer deploy
echo "🐳 Fazendo deploy dos containers..."
ssh $VPS_USER@$VPS_IP << 'ENDSSH'
cd /opt/jarvis

# Parar containers antigos (se existirem)
docker-compose down 2>/dev/null || true

# Iniciar novos containers
docker-compose up -d

# Aguardar containers iniciarem
echo ""
echo "⏳ Aguardando containers iniciarem..."
sleep 5

# Verificar status
echo ""
echo "📊 Status dos containers:"
docker-compose ps

echo ""
echo "✅ Deploy concluído!"
ENDSSH

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "✅ Deploy concluído com sucesso!"
echo ""
echo "📋 Serviços disponíveis via Tailscale:"
echo ""
echo "  🗄️  Qdrant (Vector DB):"
echo "     HTTP: http://$VPS_IP:6333"
echo "     Dashboard: http://$VPS_IP:6333/dashboard"
echo ""
echo "  🐘 PostgreSQL:"
echo "     Host: $VPS_IP:5432"
echo "     Database: jarvis"
echo "     User: jarvis"
echo ""
echo "  💾 Redis:"
echo "     Host: $VPS_IP:6379"
echo ""
echo "  🤖 Ollama:"
echo "     API: http://$VPS_IP:11434"
echo ""
echo "  🌐 Adminer (DB UI):"
echo "     URL: http://$VPS_IP:8080"
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "💡 Próximos passos:"
echo "  1. Verifique se os serviços estão rodando:"
echo "     ssh $VPS_USER@$VPS_IP 'cd /opt/jarvis && docker-compose ps'"
echo ""
echo "  2. Teste a conexão com Qdrant:"
echo "     curl http://$VPS_IP:6333/collections"
echo ""
echo "  3. Atualize seu config.toml:"
echo "     cat config.toml.vps >> ~/.jarvis/config.toml"
echo ""
