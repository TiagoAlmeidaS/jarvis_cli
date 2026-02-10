#!/bin/bash
# Script para iniciar o Qdrant via Docker

echo "🚀 Iniciando Qdrant Vector Database..."

# Verificar se Docker está rodando
if ! docker info > /dev/null 2>&1; then
    echo "❌ Docker não está rodando. Por favor, inicie o Docker Desktop."
    exit 1
fi

# Iniciar Qdrant
docker run -d \
    --name jarvis-qdrant \
    -p 6333:6333 \
    -p 6334:6334 \
    -v "$(pwd)/qdrant_storage:/qdrant/storage" \
    qdrant/qdrant:latest

echo ""
echo "✅ Qdrant iniciado com sucesso!"
echo "📊 Dashboard: http://localhost:6333/dashboard"
echo "🔌 API: http://localhost:6333"
echo ""
echo "Para parar: docker stop jarvis-qdrant"
echo "Para remover: docker rm jarvis-qdrant"
