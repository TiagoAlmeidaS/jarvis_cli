#!/bin/bash
# Script de teste com logs detalhados

echo "🔍 Teste Databricks com Logs Detalhados"
echo "========================================"
echo ""

# Carregar credenciais
source ./configure-credentials.sh 2>/dev/null || true

cd jarvis-rs

# Ativar TODOS os logs relevantes
export RUST_LOG="jarvis_api=trace,jarvis_client=debug,reqwest=debug"
export RUST_BACKTRACE=1

echo "📋 Logs ativados:"
echo "  - jarvis_api=trace (mostra payloads)"
echo "  - jarvis_client=debug (mostra requests)"
echo "  - reqwest=debug (mostra HTTP)"
echo ""

echo "🚀 Executando Jarvis..."
echo ""

# Usar debug build
./target/debug/jarvis.exe chat -c model_provider=databricks -m databricks-claude-opus-4-5
