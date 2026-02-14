#!/bin/bash
# ============================================================================
# Run Script for Jarvis CLI with Ollama Integration
# ============================================================================
# Usage: ./scripts/run-jarvis.sh [model] [mode]
#   model: llama3.2:3b (default), gemma2:2b, qwen2.5:7b, etc.
#   mode: chat (default), exec
#
# Examples:
#   ./scripts/run-jarvis.sh                          # Chat com llama3.2:3b
#   ./scripts/run-jarvis.sh gemma2:2b                # Chat com gemma2:2b
#   ./scripts/run-jarvis.sh llama3.1:8b exec "Hello" # Exec mode
# ============================================================================

set -e  # Exit on error

# Navegar para a raiz do projeto (pai do diretório scripts/)
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR/.."

# Configurações
DEFAULT_MODEL="llama3.2:3b"
DEFAULT_MODE="chat"
OLLAMA_VPS="http://100.98.213.86:11434/v1"

MODEL="${1:-$DEFAULT_MODEL}"
MODE="${2:-$DEFAULT_MODE}"
PROMPT="${3:-}"

# Cores para output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo "========================================"
echo "🚀 Jarvis CLI + Ollama VPS"
echo "========================================"
echo ""

# Verificar se o binário existe
BINARY_DEBUG="jarvis-rs/target/debug/jarvis.exe"
BINARY_RELEASE="jarvis-rs/target/release/jarvis.exe"

if [ -f "$BINARY_RELEASE" ]; then
    BINARY="$BINARY_RELEASE"
    BUILD_TYPE="release"
elif [ -f "$BINARY_DEBUG" ]; then
    BINARY="$BINARY_DEBUG"
    BUILD_TYPE="debug"
else
    echo -e "${RED}❌ Erro: Binário não encontrado!${NC}"
    echo ""
    echo -e "${YELLOW}Execute primeiro:${NC}"
    echo "  ./scripts/build-jarvis.sh"
    echo ""
    exit 1
fi

echo -e "${GREEN}✓ Binário encontrado:${NC} $BINARY ($BUILD_TYPE)"
echo -e "${GREEN}✓ Ollama VPS:${NC} $OLLAMA_VPS"
echo -e "${GREEN}✓ Modelo:${NC} $MODEL"
echo -e "${GREEN}✓ Modo:${NC} $MODE"
echo ""

# Verificar conectividade com Ollama VPS
echo -e "${BLUE}🔍 Verificando conexão com Ollama VPS...${NC}"
if curl -s --connect-timeout 5 "http://100.98.213.86:11434/api/tags" > /dev/null 2>&1; then
    echo -e "${GREEN}✓ Ollama VPS acessível${NC}"
else
    echo -e "${YELLOW}⚠️  Aviso: Ollama VPS não está respondendo${NC}"
    echo -e "${YELLOW}   Verifique se:${NC}"
    echo "     1. Tailscale está conectado"
    echo "     2. Ollama está rodando na VPS"
    echo "     3. IP está correto (100.98.213.86)"
    echo ""
    read -p "Continuar mesmo assim? (y/N): " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 1
    fi
fi

echo ""
echo "========================================"
echo -e "${BLUE}🎯 Iniciando Jarvis...${NC}"
echo "========================================"
echo ""

# Configurar variável de ambiente
export OLLAMA_BASE_URL="$OLLAMA_VPS"

# Executar conforme o modo
if [ "$MODE" = "exec" ]; then
    if [ -z "$PROMPT" ]; then
        echo -e "${RED}❌ Erro: Prompt é necessário para modo exec${NC}"
        echo ""
        echo -e "${YELLOW}Exemplo:${NC}"
        echo "  ./scripts/run-jarvis.sh llama3.2:3b exec \"Olá, como está?\""
        echo ""
        exit 1
    fi

    echo -e "${BLUE}Executando comando único...${NC}"
    echo ""
    ./"$BINARY" exec -c "model_provider=\"ollama\"" -m "$MODEL" --cd "$SCRIPT_DIR/.." "$PROMPT"

elif [ "$MODE" = "chat" ]; then
    echo -e "${BLUE}🎨 Abrindo modo chat interativo...${NC}"
    echo ""
    echo -e "${YELLOW}Dicas:${NC}"
    echo "  • Ctrl+C para sair"
    echo "  • Primeira resposta pode demorar ~20-30s (warm-up)"
    echo "  • Respostas seguintes serão rápidas"
    echo ""
    echo "========================================"
    echo ""

    ./"$BINARY" -c "model_provider=\"ollama\"" -m "$MODEL" --cd "$SCRIPT_DIR/.."

else
    echo -e "${RED}❌ Erro: Modo inválido: $MODE${NC}"
    echo ""
    echo -e "${YELLOW}Modos válidos:${NC}"
    echo "  • chat - Modo interativo (padrão)"
    echo "  • exec - Executar comando único"
    echo ""
    exit 1
fi
