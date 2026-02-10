#!/bin/bash
# Script para verificar o status da compilação

echo "🔍 Verificando status da compilação..."
echo ""

# Verificar se cargo está rodando
CARGO_RUNNING=$(ps aux 2>/dev/null | grep -i cargo | grep -v grep | wc -l)

if [ $CARGO_RUNNING -gt 0 ]; then
    echo "⏳ Compilação em andamento..."
    echo "   Processos cargo ativos: $CARGO_RUNNING"
    echo ""
    echo "💡 Aguarde a compilação terminar ou pressione Ctrl+C para cancelar"
else
    echo "✅ Nenhuma compilação em andamento"
    echo ""

    # Verificar se os binários existem
    echo "📦 Binários disponíveis:"

    if [ -f "jarvis-rs/target/release/jarvis.exe" ]; then
        SIZE=$(ls -lh jarvis-rs/target/release/jarvis.exe | awk '{print $5}')
        echo "  ✅ Release: jarvis-rs/target/release/jarvis.exe ($SIZE)"
    else
        echo "  ❌ Release: não encontrado"
    fi

    if [ -f "jarvis-rs/target/debug/jarvis.exe" ]; then
        SIZE=$(ls -lh jarvis-rs/target/debug/jarvis.exe | awk '{print $5}')
        echo "  ✅ Debug: jarvis-rs/target/debug/jarvis.exe ($SIZE)"
    else
        echo "  ❌ Debug: não encontrado"
    fi

    echo ""
    echo "🚀 Para testar:"
    echo "   cd jarvis-rs"
    echo "   ./target/release/jarvis.exe chat"
    echo ""
    echo "   ou"
    echo ""
    echo "   ./test-databricks.sh"
fi
