#!/bin/bash
# Script para indexar o projeto Jarvis no contexto RAG

set -e

echo "📚 Indexando Projeto Jarvis no Contexto RAG..."
echo ""

cd jarvis-rs

# Verificar se Qdrant está rodando
if ! curl -s http://localhost:6333/collections > /dev/null 2>&1; then
    echo "⚠️  Qdrant não está rodando em localhost:6333"
    echo ""
    echo "Por favor, inicie o Qdrant primeiro:"
    echo "  ./start-qdrant.sh"
    echo ""
    exit 1
fi

echo "✅ Qdrant está rodando"
echo ""

# Função para adicionar arquivo
add_file() {
    local file=$1
    local type=$2
    local lang=${3:-""}

    if [ -f "$file" ]; then
        echo "  📄 Indexando: $file"
        if [ -n "$lang" ]; then
            ./target/debug/jarvis.exe context add "$file" --doc-type "$type" --language "$lang" -o json > /dev/null 2>&1 || true
        else
            ./target/debug/jarvis.exe context add "$file" --doc-type "$type" -o json > /dev/null 2>&1 || true
        fi
    fi
}

# Documentação raiz
echo "📖 Indexando documentação principal..."
add_file "../README.md" "markdown"
add_file "../QUICK_START.md" "markdown"
add_file "../GUIA_CONFIGURACAO.md" "markdown"
add_file "../COMO_TESTAR.md" "markdown"

# Documentação do jarvis-rs
echo ""
echo "📖 Indexando documentação do jarvis-rs..."
add_file "./README.md" "markdown"
add_file "./INTEGRATION_TESTS.md" "markdown"

# Arquivos de configuração
echo ""
echo "⚙️  Indexando arquivos de configuração..."
add_file "../config.toml.example" "project"
add_file "./Cargo.toml" "project"
add_file "./cli/Cargo.toml" "project"

# Código-fonte principal (apenas arquivos chave)
echo ""
echo "💻 Indexando código-fonte principal..."
add_file "./cli/src/main.rs" "code" "rust"
add_file "./cli/src/lib.rs" "code" "rust"

# Documentação de features
echo ""
echo "📋 Indexando documentação de features..."
if [ -d "../docs/features" ]; then
    for file in ../docs/features/*.md; do
        if [ -f "$file" ]; then
            add_file "$file" "markdown"
        fi
    done
fi

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "✅ Indexação concluída!"
echo ""

# Mostrar estatísticas
echo "📊 Estatísticas do contexto:"
./target/debug/jarvis.exe context stats

echo ""
echo "💡 Para buscar no contexto:"
echo "   ./target/debug/jarvis.exe context search \"sua query\" --limit 5"
echo ""
echo "💡 Para listar documentos:"
echo "   ./target/debug/jarvis.exe context list"
echo ""
