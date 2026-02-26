#!/bin/bash
# Script para executar Jarvis CLI com estratégia Free
# Uso: ./scripts/run-jarvis-free.sh

MODEL="${1:-openrouter/free}"
PROVIDER="${2:-openrouter}"
PROMPT="${3:-}"

echo "============================================================"
echo "  Jarvis CLI - Modo Free"
echo "============================================================"
echo ""

# Verificar se estamos no diretório correto
if [ ! -d "jarvis-rs" ]; then
    echo "ERRO: Execute este script da raiz do projeto (jarvis_cli)"
    exit 1
fi

# Verificar variáveis de ambiente
if [ "$PROVIDER" = "openrouter" ] && [ -z "$OPENROUTER_API_KEY" ]; then
    echo "AVISO: OPENROUTER_API_KEY não configurada!"
    echo "Configure com: export OPENROUTER_API_KEY='sua-chave'"
    echo "Obtenha em: https://openrouter.ai/keys"
    echo ""
fi

if [ "$PROVIDER" = "google" ] && [ -z "$GOOGLE_API_KEY" ]; then
    echo "AVISO: GOOGLE_API_KEY não configurada!"
    echo "Configure com: export GOOGLE_API_KEY='sua-chave'"
    echo "Obtenha em: https://ai.google.dev"
    echo ""
fi

cd jarvis-rs || exit 1

# Verificar se o binário existe
if [ -f "target/release/jarvis" ]; then
    JARVIS_EXE="./target/release/jarvis"
elif [ -f "target/debug/jarvis" ]; then
    JARVIS_EXE="./target/debug/jarvis"
else
    echo "ERRO: Jarvis não compilado. Execute primeiro:"
    echo "  cargo build --package jarvis-cli --release"
    cd ..
    exit 1
fi

echo "Provider: $PROVIDER"
echo "Model: $MODEL"
echo ""

# Executar Jarvis
if [ -n "$PROMPT" ]; then
    echo "Executando comando não-interativo..."
    echo ""
    "$JARVIS_EXE" exec -c "model_provider=$PROVIDER" -m "$MODEL" "$PROMPT"
else
    echo "Iniciando chat interativo..."
    echo "Pressione Ctrl+C para sair"
    echo ""
    "$JARVIS_EXE" chat -c "model_provider=$PROVIDER" -m "$MODEL"
fi

cd ..
