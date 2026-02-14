#!/bin/bash
# ============================================================================
# Build Script for Jarvis CLI
# ============================================================================
# Usage: ./build-jarvis.sh [release|debug]
# Default: debug
# ============================================================================

set -e  # Exit on error

BUILD_TYPE="${1:-debug}"

echo "========================================"
echo "🔨 Building Jarvis CLI"
echo "========================================"
echo ""

# Cores para output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Verificar se estamos no diretório correto
if [ ! -d "jarvis-rs" ]; then
    echo -e "${RED}❌ Erro: Diretório jarvis-rs não encontrado!${NC}"
    echo -e "${YELLOW}Execute este script da raiz do projeto${NC}"
    exit 1
fi

echo -e "${BLUE}📂 Diretório: $(pwd)${NC}"
echo -e "${BLUE}🔧 Tipo de build: $BUILD_TYPE${NC}"
echo ""

# Navegar para o diretório do Rust
cd jarvis-rs

# Limpar builds anteriores (opcional)
read -p "🧹 Limpar builds anteriores? (y/N): " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    echo -e "${YELLOW}Limpando target...${NC}"
    cargo clean
    echo -e "${GREEN}✓ Limpeza concluída${NC}"
    echo ""
fi

# Executar build
echo -e "${YELLOW}🔨 Compilando...${NC}"
echo ""

START_TIME=$(date +%s)

if [ "$BUILD_TYPE" = "release" ]; then
    echo -e "${BLUE}Building RELEASE (otimizado, pode levar tempo)...${NC}"
    cargo build --release --bin jarvis
    BINARY_PATH="target/release/jarvis.exe"
else
    echo -e "${BLUE}Building DEBUG (mais rápido, sem otimizações)...${NC}"
    cargo build --bin jarvis
    BINARY_PATH="target/debug/jarvis.exe"
fi

END_TIME=$(date +%s)
DURATION=$((END_TIME - START_TIME))
MINUTES=$((DURATION / 60))
SECONDS=$((DURATION % 60))

# Voltar ao diretório raiz
cd ..

# Verificar se o binário foi criado
if [ -f "jarvis-rs/$BINARY_PATH" ]; then
    BINARY_SIZE=$(du -h "jarvis-rs/$BINARY_PATH" | cut -f1)
    echo ""
    echo "========================================"
    echo -e "${GREEN}✅ Build concluído com sucesso!${NC}"
    echo "========================================"
    echo ""
    echo -e "${GREEN}📦 Binário criado:${NC} jarvis-rs/$BINARY_PATH"
    echo -e "${GREEN}💾 Tamanho:${NC} $BINARY_SIZE"
    echo -e "${GREEN}⏱️  Tempo:${NC} ${MINUTES}m ${SECONDS}s"
    echo ""
    echo -e "${BLUE}Para executar:${NC}"
    echo "  ./run-jarvis.sh"
    echo ""
    echo -e "${BLUE}Ou manualmente:${NC}"
    echo "  export OLLAMA_BASE_URL=\"http://100.98.213.86:11434/v1\""
    echo "  ./jarvis-rs/$BINARY_PATH -c 'model_provider=\"ollama\"' -m llama3.2:3b"
    echo ""
else
    echo ""
    echo "========================================"
    echo -e "${RED}❌ Build falhou!${NC}"
    echo "========================================"
    echo ""
    echo -e "${YELLOW}Verifique os erros acima.${NC}"
    echo ""
    exit 1
fi
