#!/bin/bash

# Script para iniciar o Jarvis Web API com Docker Compose

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

cd "$PROJECT_ROOT"

echo "🚀 Iniciando Jarvis Web API com Docker Compose..."
echo ""

# Verificar se .env existe
if [ ! -f .env ]; then
    echo "⚠️  Arquivo .env não encontrado!"
    echo "📝 Criando .env a partir de .env.example..."
    
    if [ -f .env.example ]; then
        cp .env.example .env
        
        # Gerar API key automaticamente
        API_KEY=$(openssl rand -hex 32)
        if [[ "$OSTYPE" == "darwin"* ]]; then
            # macOS
            sed -i '' "s/your-api-key-here/$API_KEY/" .env
        else
            # Linux
            sed -i "s/your-api-key-here/$API_KEY/" .env
        fi
        
        echo "✅ Arquivo .env criado!"
        echo "🔑 API Key gerada: $API_KEY"
        echo ""
    else
        echo "❌ Arquivo .env.example não encontrado!"
        echo "📝 Criando .env básico..."
        cat > .env << EOF
JARVIS_API_KEY=$(openssl rand -hex 32)
WEB_API_PORT=3000
RUST_LOG=info
EOF
        echo "✅ Arquivo .env criado!"
    fi
    echo ""
fi

# Verificar se Docker está rodando
if ! docker info > /dev/null 2>&1; then
    echo "❌ Docker não está rodando!"
    echo "   Por favor, inicie o Docker Desktop e tente novamente."
    exit 1
fi

# Verificar se docker-compose está disponível
if command -v docker-compose &> /dev/null; then
    COMPOSE_CMD="docker-compose"
elif docker compose version &> /dev/null; then
    COMPOSE_CMD="docker compose"
else
    echo "❌ docker-compose não encontrado!"
    exit 1
fi

# Subir os serviços
echo "🐳 Subindo containers..."
$COMPOSE_CMD -f docker-compose.web-api.yml up -d

echo ""
echo "⏳ Aguardando serviço iniciar..."
sleep 5

# Verificar saúde
if docker exec jarvis-web-api wget --quiet --tries=1 --spider http://localhost:3000/api/health 2>/dev/null; then
    echo ""
    echo "✅ Jarvis Web API está rodando!"
    echo ""
    echo "📊 Informações:"
    echo "   - URL: http://localhost:${WEB_API_PORT:-3000}"
    echo "   - Health: http://localhost:${WEB_API_PORT:-3000}/api/health"
    echo "   - Logs: $COMPOSE_CMD -f docker-compose.web-api.yml logs -f"
    echo "   - Parar: $COMPOSE_CMD -f docker-compose.web-api.yml down"
    echo ""
    
    # Mostrar API key do .env
    if [ -f .env ]; then
        API_KEY=$(grep JARVIS_API_KEY .env | cut -d '=' -f2)
        if [ -n "$API_KEY" ]; then
            echo "🔑 API Key: $API_KEY"
            echo ""
        fi
    fi
else
    echo ""
    echo "⚠️  Serviço iniciado, mas health check ainda não passou."
    echo "   Verifique os logs: $COMPOSE_CMD -f docker-compose.web-api.yml logs -f"
    echo ""
fi
