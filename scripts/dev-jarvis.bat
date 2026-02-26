@echo off
REM ============================================================================
REM Dev Script for Jarvis CLI (Windows)
REM ============================================================================
REM Compiles and runs Jarvis in a single step using cargo run (incremental build).
REM Much faster for development than separate build + run.
REM
REM Usage: dev-jarvis.bat [provider] [model]
REM
REM Providers:
REM   (default)  - Google Gemini (free with daily quotas, best for development)
REM   free       - OpenRouter Free Strategy
REM   openrouter - OpenRouter API (same as free, explicit)
REM   free_google - Google AI Studio Free (same as default, explicit)
REM   qwen       - OpenRouter + Qwen3 Coder 80B MoE (best cheap paid for TUI, ~$0.09/day)
REM   nemo       - OpenRouter + Mistral Nemo 12B (daemon only, no tool calling)
REM   agent      - AgentLoop + OpenRouter + Mistral Nemo (text-based tool calling)
REM   gemini     - Google AI Studio (free tier, alias for free_google)
REM   ollama     - Ollama local/VPS
REM   azure      - Azure OpenAI
REM
REM Options:
REM   --release    Build in release mode (slower build, faster runtime)
REM
REM ---- OpenRouter Models Reference ----
REM
REM Free models (tested with system messages + streaming):
REM   openrouter/free                       - Default free model (free strategy default)
REM   Note: When using model_provider=openrouter, use format: openrouter/free
REM   (without the -0528 suffix and without openrouter/ prefix)
REM   nvidia/nemotron-nano-9b-v2:free       - Fast and light
REM   stepfun/step-3.5-flash:free           - Fast, good general purpose
REM   z-ai/glm-4.5-air:free                - Good general purpose
REM   google/gemini-2.5-flash              - Google AI Studio free (requires GOOGLE_API_KEY)
REM   google/gemini-2.5-flash-lite          - Google AI Studio free (faster)
REM
REM Cheap paid models (best cost-benefit for content):
REM   mistralai/mistral-nemo                - 12B, multilingual, $0.02-0.04/M (nemo default)
REM   meta-llama/llama-3.1-8b-instruct     - 8B, $0.02-0.05/M
REM   nousresearch/deephermes-3-mistral-24b-preview - 24B reasoning, $0.02-0.10/M
REM   qwen/qwen3-coder-next                - 80B MoE, $0.07-0.30/M
REM   z-ai/glm-4.7-flash                   - 30B, $0.06-0.40/M
REM
REM IMPORTANT: google/gemma-* models do NOT support system messages
REM            and will fail with Jarvis (which always sends system prompt).
REM
REM Examples:
REM   dev-jarvis.bat                                       (Default - Google Gemini free)
REM   dev-jarvis.bat free                                  (Free strategy - explicit)
REM   dev-jarvis.bat free stepfun/step-3.5-flash:free     (Free strategy - custom model)
REM   dev-jarvis.bat free_google                           (Google AI Studio free)
REM   dev-jarvis.bat free_google gemini-2.5-flash-lite      (Google AI Studio - faster model)
REM   dev-jarvis.bat qwen                                  (OpenRouter + Qwen3 Coder, paid ~$0.09/day)
REM   dev-jarvis.bat agent                                 (AgentLoop + OpenRouter + Mistral Nemo)
REM   dev-jarvis.bat agent mistralai/codestral-mamba-latest (AgentLoop + custom model)
REM   dev-jarvis.bat openrouter qwen/qwen3-coder-next     (OpenRouter explicit)
REM   dev-jarvis.bat openrouter nvidia/nemotron-nano-9b-v2:free
REM   dev-jarvis.bat openrouter stepfun/step-3.5-flash:free
REM   dev-jarvis.bat gemini gemini-2.5-flash-lite          (Google AI Studio - alias)
REM   dev-jarvis.bat ollama llama3.2:3b
REM   dev-jarvis.bat azure gpt-4o
REM   dev-jarvis.bat --release free                        (Release build with free strategy)
REM ============================================================================

setlocal enabledelayedexpansion

REM Navigate to project root (parent of scripts/)
cd /d "%~dp0.."

REM Defaults - Google Gemini (free with daily quotas)
set "PROVIDER=google"
set "MODEL=gemini-2.5-flash"
set "CARGO_FLAGS="
set "FREE_STRATEGY=1"

REM Load .env file if it exists
if exist ".env" (
    for /f "usebackq tokens=1,* delims==" %%A in (".env") do (
        set "LINE=%%A"
        if not "!LINE:~0,1!"=="#" (
            if not "%%A"=="" set "%%A=%%B"
        )
    )
)

REM Parse arguments
:parse_args
if "%1"=="" goto :done_args
if "%1"=="--release" (
    set "CARGO_FLAGS=--release"
    shift
    goto :parse_args
)
if "%1"=="free" (
    set "PROVIDER=openrouter"
    set "MODEL=openrouter/free"
    set "FREE_STRATEGY=1"
    shift
    goto :parse_args
)
if "%1"=="free_google" (
    set "PROVIDER=google"
    set "MODEL=gemini-2.5-flash"
    set "FREE_STRATEGY=1"
    shift
    goto :parse_args
)
if "%1"=="openrouter" (
    set "PROVIDER=openrouter"
    set "MODEL=openrouter/free"
    set "FREE_STRATEGY=1"
    shift
    goto :parse_args
)
if "%1"=="nemo" (
    set "PROVIDER=openrouter"
    set "MODEL=mistralai/mistral-nemo"
    shift
    goto :parse_args
)
if "%1"=="agent" (
    set "PROVIDER=openrouter"
    set "MODEL=mistralai/mistral-nemo"
    set "AGENT_LOOP=1"
    shift
    goto :parse_args
)
if "%1"=="qwen" (
    set "PROVIDER=openrouter"
    set "MODEL=qwen/qwen3-coder-next"
    shift
    goto :parse_args
)
if "%1"=="gemini" (
    set "PROVIDER=google"
    set "MODEL=gemini-2.5-flash-lite"
    set "FREE_STRATEGY=1"
    shift
    goto :parse_args
)
if "%1"=="ollama" (
    set "PROVIDER=ollama"
    set "MODEL=llama3.2:3b"
    set "OLLAMA_BASE_URL=http://100.98.213.86:11434/v1"
    shift
    goto :parse_args
)
if "%1"=="azure" (
    set "PROVIDER=azure-openai"
    set "MODEL=gpt-4o"
    shift
    goto :parse_args
)
REM Anything else is a model name
set "MODEL=%1"
shift
goto :parse_args
:done_args

echo ========================================
echo  Jarvis CLI - Dev Mode
echo ========================================
echo.
echo  Provider: %PROVIDER%
echo  Modelo:   %MODEL%
if defined FREE_STRATEGY (
    echo  Strategy: Free [using free models]
)
if defined AGENT_LOOP (
    echo  Mode:     AgentLoop [text-based tool calling]
)
if defined CARGO_FLAGS (
    echo  Build:    release
) else (
    echo  Build:    debug [incremental]
)
echo.
if defined FREE_STRATEGY (
    if "%PROVIDER%"=="openrouter" (
        if not defined OPENROUTER_API_KEY (
            echo  AVISO: OPENROUTER_API_KEY nao configurada!
            echo  Configure com: set OPENROUTER_API_KEY=sk-or-v1-...
            echo  Obtenha em: https://openrouter.ai/keys
            echo.
        )
    )
    if "%PROVIDER%"=="google" (
        if not defined GOOGLE_API_KEY (
            echo  AVISO: GOOGLE_API_KEY nao configurada!
            echo  Configure com: set GOOGLE_API_KEY=...
            echo  Obtenha em: https://ai.google.dev
            echo.
        )
    )
)
echo  Compilando e executando...
echo ========================================
echo.

REM Build the config overrides
set "CONFIG_OVERRIDES=model_provider=\"%PROVIDER%\""

REM When AgentLoop mode is requested, inject [agent_loop] config overrides.
REM The api_key is auto-resolved from OPENROUTER_API_KEY env var via base_url detection.
if defined AGENT_LOOP (
    set "CONFIG_OVERRIDES=%CONFIG_OVERRIDES%" -c "agent_loop.mode=\"text_based\"" -c "agent_loop.base_url=\"https://openrouter.ai/api/v1\"" -c "agent_loop.model=\"%MODEL%\""
)

REM Use cargo run for incremental compilation + execution in one step
REM --cd points Jarvis to the project root (jarvis_cli), not the Rust subfolder
cd jarvis-rs
if defined AGENT_LOOP (
    cargo run %CARGO_FLAGS% --bin jarvis -- -c "model_provider=\"%PROVIDER%\"" -c "agent_loop.mode=\"text_based\"" -c "agent_loop.base_url=\"https://openrouter.ai/api/v1\"" -c "agent_loop.model=\"%MODEL%\"" -m "%MODEL%" --cd "%~dp0.."
) else (
    cargo run %CARGO_FLAGS% --bin jarvis -- -c "model_provider=\"%PROVIDER%\"" -m "%MODEL%" --cd "%~dp0.."
)
set "EXIT_CODE=%ERRORLEVEL%"
cd ..

if %EXIT_CODE% neq 0 (
    echo.
    echo  Jarvis encerrou com codigo %EXIT_CODE%
    pause
    exit /b %EXIT_CODE%
)

endlocal
