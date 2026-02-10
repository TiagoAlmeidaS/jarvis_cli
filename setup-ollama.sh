#!/bin/bash
# Setup Ollama para Desenvolvimento com Jarvis

echo "🚀 Setup Ollama para Desenvolvimento"
echo "====================================="
echo ""

# Verificar se Ollama está instalado
if ! command -v ollama &> /dev/null; then
    echo "❌ Ollama não encontrado!"
    echo ""
    echo "📥 Instale o Ollama:"
    echo "   Windows: https://ollama.com/download"
    echo "   ou: winget install Ollama.Ollama"
    echo ""
    exit 1
fi

echo "✅ Ollama instalado: $(ollama --version)"
echo ""

# Verificar se o servidor está rodando
if ! curl -s http://localhost:11434/api/tags &> /dev/null; then
    echo "⚠️  Servidor Ollama não está rodando"
    echo "   Iniciando..."
    ollama serve &
    sleep 3
fi

echo "✅ Servidor Ollama rodando"
echo ""

# Listar modelos instalados
echo "📦 Modelos instalados:"
ollama list
echo ""

# Sugerir modelos para desenvolvimento
echo "💡 Modelos recomendados para desenvolvimento:"
echo ""
echo "   🏃 Ultra-rápido (256MB):"
echo "      ollama pull phi3:mini"
echo ""
echo "   ⚡ Rápido e bom (2GB):"
echo "      ollama pull llama3.2:3b"
echo ""
echo "   🎯 Balanceado (4.7GB):"
echo "      ollama pull llama3.1:8b"
echo ""
echo "   💻 Para código (3.8GB):"
echo "      ollama pull codellama:7b"
echo ""
echo "   🧠 Mais capaz (40GB):"
echo "      ollama pull llama3.1:70b"
echo ""

# Perguntar se quer baixar modelos
read -p "Baixar llama3.2:3b (2GB - recomendado)? (y/n) " -n 1 -r
echo ""
if [[ $REPLY =~ ^[Yy]$ ]]; then
    echo "📥 Baixando llama3.2:3b..."
    ollama pull llama3.2:3b
    echo "✅ Modelo baixado!"
fi

echo ""
echo "🎉 Setup concluído!"
echo ""
echo "🚀 Para testar com Jarvis:"
echo "   cd jarvis-rs"
echo "   ./target/debug/jarvis.exe chat -c model_provider=ollama -m llama3.2:3b"
echo ""
