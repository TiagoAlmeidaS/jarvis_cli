#!/bin/bash
# Script para limpar arquivos do Git que estão no .gitignore

echo "🧹 Limpando arquivos do Git tracking..."
echo ""

cd "$(dirname "$0")"

# Arquivos para remover do tracking (mas manter localmente)
FILES_TO_REMOVE=(
    # Documentação temporária
    "ANALYTICS_IMPLEMENTATION_SUMMARY.md"
    "BUG_CRITICO_CORRIGIDO.md"
    "COMPILACAO_SUCESSO.md"
    "DATABRICKS_URL_FIX.md"
    "ERRO_COMPILACAO_CLOUD_TASKS.md"
    "IMPLEMENTACAO_SKILLS_PROGRESSO.md"
    "IMPLEMENTATION_SUCCESS_REPORT.md"
    "PROBLEMA_IDENTIFICADO.md"
    "QDRANT_IMPLEMENTATION_SUMMARY.md"
    "RESUMO_TRABALHO_TESTES.md"
    "SOLUCAO_FINAL.md"
    "TESTING_IMPLEMENTATION_SUMMARY.md"
    "TESTING_PROGRESS.md"
    "jarvis-cli/RESUMO_CORRECOES.md"
    "jarvis-cli/RESUMO_FINAL.md"

    # Scripts de teste temporários
    "BUILD_AND_RUN_COMPLETE.sh"
    "TEST_DATABRICKS_COMPLETE.sh"
    "TEST_NOW.sh"
    "BUILD_AND_RUN.sh"
    "RUN_JARVIS.sh"
    "bashrc-snippet.sh"

    # Arquivos de configuração com credenciais
    "configure-credentials.ps1"

    # Outros arquivos temporários
    "VPS_INFO.txt"
    "test_small.md"
    "appsettings.Example.json"
)

echo "📋 Arquivos a serem removidos do Git (mantidos localmente):"
echo ""

for file in "${FILES_TO_REMOVE[@]}"; do
    if [ -f "$file" ]; then
        echo "  - $file"
    fi
done

echo ""
read -p "Continuar? (y/n) " -n 1 -r
echo ""

if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "❌ Cancelado"
    exit 1
fi

echo ""
echo "🔄 Removendo arquivos do Git tracking..."
echo ""

for file in "${FILES_TO_REMOVE[@]}"; do
    if [ -f "$file" ]; then
        git rm --cached "$file" 2>/dev/null && echo "✓ $file" || echo "⚠ $file (já removido ou não existe)"
    fi
done

echo ""
echo "✅ Limpeza concluída!"
echo ""
echo "📝 Próximos passos:"
echo "   1. Verificar mudanças: git status"
echo "   2. Commit: git add .gitignore && git commit -m 'chore: update .gitignore and remove temp files'"
echo "   3. Os arquivos ainda existem localmente, apenas não serão mais trackeados"
echo ""
