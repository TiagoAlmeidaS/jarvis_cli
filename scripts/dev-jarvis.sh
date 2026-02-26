#!/bin/bash
# ============================================================================
# Dev Script for Jarvis CLI (Git Bash / Linux / Mac)
# ============================================================================
# Wrapper script that calls the Windows batch file or runs directly on Linux/Mac
#
# Usage: ./dev-jarvis.sh [provider] [model]
#
# Providers:
#   (default)  - Google Gemini (free with daily quotas, best for development)
#   free       - OpenRouter Free Strategy
#   openrouter - OpenRouter API (same as free, explicit)
#   free_google - Google AI Studio Free (same as default, explicit)
#   qwen       - OpenRouter + Qwen3 Coder 80B MoE (best cheap paid for TUI, ~$0.09/day)
#   nemo       - OpenRouter + Mistral Nemo 12B (daemon only, no tool calling)
#   agent      - AgentLoop + OpenRouter + Mistral Nemo (text-based tool calling)
#   gemini     - Google AI Studio (free tier, alias for free_google)
#   ollama     - Ollama local/VPS
#   azure      - Azure OpenAI
#
# Options:
#   --release    Build in release mode (slower build, faster runtime)
# ============================================================================

set -e  # Exit on error

# Navigate to project root (parent of scripts/)
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR/.."

# Check if we're on Windows (Git Bash or WSL)
if [[ "$OSTYPE" == "msys" ]] || [[ "$OSTYPE" == "cygwin" ]] || [[ -n "$WSL_DISTRO_NAME" ]]; then
    # On Windows/Git Bash, try to use the .bat file
    if [ -f "scripts/dev-jarvis.bat" ]; then
        echo "Executando via Windows Batch (dev-jarvis.bat)..."
        echo ""
        # Convert arguments to Windows format and execute
        cmd.exe //c "scripts\\dev-jarvis.bat" "$@"
        exit $?
    fi
fi

# On Linux/Mac, run directly with cargo
echo "========================================"
echo "  Jarvis CLI - Dev Mode (Native)"
echo "========================================"
echo ""

# Defaults - Google Gemini (free with daily quotas)
PROVIDER="google"
MODEL="gemini-2.5-flash"
CARGO_FLAGS=""
FREE_STRATEGY=1
AGENT_LOOP=""

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --release)
            CARGO_FLAGS="--release"
            shift
            ;;
        free)
            PROVIDER="openrouter"
            MODEL="openrouter/free"
            FREE_STRATEGY=1
            shift
            ;;
        free_google)
            PROVIDER="google"
            MODEL="gemini-2.5-flash"
            FREE_STRATEGY=1
            shift
            ;;
        openrouter)
            PROVIDER="openrouter"
            MODEL="openrouter/free"
            FREE_STRATEGY=1
            shift
            ;;
        nemo)
            PROVIDER="openrouter"
            MODEL="mistralai/mistral-nemo"
            FREE_STRATEGY=""
            shift
            ;;
        agent)
            PROVIDER="openrouter"
            MODEL="mistralai/mistral-nemo"
            AGENT_LOOP=1
            FREE_STRATEGY=""
            shift
            ;;
        qwen)
            PROVIDER="openrouter"
            MODEL="qwen/qwen3-coder-next"
            FREE_STRATEGY=""
            shift
            ;;
        gemini)
            PROVIDER="google"
            MODEL="gemini-2.5-flash-lite"
            FREE_STRATEGY=1
            shift
            ;;
        ollama)
            PROVIDER="ollama"
            MODEL="llama3.2:3b"
            export OLLAMA_BASE_URL="http://100.98.213.86:11434/v1"
            FREE_STRATEGY=""
            shift
            ;;
        azure)
            PROVIDER="azure-openai"
            MODEL="gpt-4o"
            FREE_STRATEGY=""
            shift
            ;;
        *)
            # Assume it's a model name
            MODEL="$1"
            shift
            ;;
    esac
done

echo "  Provider: $PROVIDER"
echo "  Modelo:   $MODEL"
if [ -n "$FREE_STRATEGY" ]; then
    echo "  Strategy: Free [using free models]"
fi
if [ -n "$AGENT_LOOP" ]; then
    echo "  Mode:     AgentLoop [text-based tool calling]"
fi
if [ -n "$CARGO_FLAGS" ]; then
    echo "  Build:    release"
else
    echo "  Build:    debug [incremental]"
fi
echo ""

# Check API keys for free strategy
if [ -n "$FREE_STRATEGY" ]; then
    if [ "$PROVIDER" = "openrouter" ] && [ -z "$OPENROUTER_API_KEY" ]; then
        echo "  AVISO: OPENROUTER_API_KEY não configurada!"
        echo "  Configure com: export OPENROUTER_API_KEY=sk-or-v1-..."
        echo "  Obtenha em: https://openrouter.ai/keys"
        echo ""
    fi
    if [ "$PROVIDER" = "google" ] && [ -z "$GOOGLE_API_KEY" ]; then
        echo "  AVISO: GOOGLE_API_KEY não configurada!"
        echo "  Configure com: export GOOGLE_API_KEY=..."
        echo "  Obtenha em: https://ai.google.dev"
        echo ""
    fi
fi

echo "  Compilando e executando..."
echo "========================================"
echo ""

# Navigate to jarvis-rs
cd jarvis-rs || exit 1

# Build command
if [ -n "$AGENT_LOOP" ]; then
    cargo run $CARGO_FLAGS --bin jarvis -- \
        -c "model_provider=\"$PROVIDER\"" \
        -c "agent_loop.mode=\"text_based\"" \
        -c "agent_loop.base_url=\"https://openrouter.ai/api/v1\"" \
        -c "agent_loop.model=\"$MODEL\"" \
        -m "$MODEL" \
        --cd "$SCRIPT_DIR/.."
else
    cargo run $CARGO_FLAGS --bin jarvis -- \
        -c "model_provider=\"$PROVIDER\"" \
        -m "$MODEL" \
        --cd "$SCRIPT_DIR/.."
fi

EXIT_CODE=$?
cd ..

if [ $EXIT_CODE -ne 0 ]; then
    echo ""
    echo "  Jarvis encerrou com código $EXIT_CODE"
    exit $EXIT_CODE
fi
