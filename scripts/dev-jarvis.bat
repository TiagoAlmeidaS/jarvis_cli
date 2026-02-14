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
REM   openrouter - OpenRouter API (default, free DeepSeek R1)
REM   qwen       - OpenRouter + Qwen3 Coder 80B MoE (best cheap paid for TUI, ~$0.09/day)
REM   nemo       - OpenRouter + Mistral Nemo 12B (daemon only, no tool calling)
REM   gemini     - Google AI Studio (free tier)
REM   ollama     - Ollama local/VPS
REM   azure      - Azure OpenAI
REM
REM Options:
REM   --release    Build in release mode (slower build, faster runtime)
REM
REM ---- OpenRouter Models Reference ----
REM
REM Free models (tested with system messages + streaming):
REM   deepseek/deepseek-r1-0528:free        - Best reasoning (default)
REM   nvidia/nemotron-nano-9b-v2:free       - Fast and light
REM   stepfun/step-3.5-flash:free           - Fast, good general purpose
REM   z-ai/glm-4.5-air:free                - Good general purpose
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
REM   dev-jarvis.bat                                       (OpenRouter + DeepSeek R1 free)
REM   dev-jarvis.bat qwen                                  (OpenRouter + Qwen3 Coder, paid ~$0.09/day)
REM   dev-jarvis.bat openrouter qwen/qwen3-coder-next     (same as above, explicit)
REM   dev-jarvis.bat openrouter nvidia/nemotron-nano-9b-v2:free
REM   dev-jarvis.bat openrouter stepfun/step-3.5-flash:free
REM   dev-jarvis.bat gemini gemini-2.5-flash-lite
REM   dev-jarvis.bat ollama llama3.2:3b
REM   dev-jarvis.bat azure gpt-4o
REM   dev-jarvis.bat --release qwen
REM ============================================================================

setlocal enabledelayedexpansion

REM Navigate to project root (parent of scripts/)
cd /d "%~dp0.."

REM Defaults
set "PROVIDER=openrouter"
set "MODEL=deepseek/deepseek-r1-0528:free"
set "CARGO_FLAGS="

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
if "%1"=="openrouter" (
    set "PROVIDER=openrouter"
    set "MODEL=deepseek/deepseek-r1-0528:free"
    shift
    goto :parse_args
)
if "%1"=="nemo" (
    set "PROVIDER=openrouter"
    set "MODEL=mistralai/mistral-nemo"
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
if defined CARGO_FLAGS (
    echo  Build:    release
) else (
    echo  Build:    debug [incremental]
)
echo.
echo  Compilando e executando...
echo ========================================
echo.

REM Use cargo run for incremental compilation + execution in one step
REM --cd points Jarvis to the project root (jarvis_cli), not the Rust subfolder
cd jarvis-rs
cargo run %CARGO_FLAGS% --bin jarvis -- -c "model_provider=\"%PROVIDER%\"" -m "%MODEL%" --cd "%~dp0.."
set "EXIT_CODE=%ERRORLEVEL%"
cd ..

if %EXIT_CODE% neq 0 (
    echo.
    echo  Jarvis encerrou com codigo %EXIT_CODE%
    pause
    exit /b %EXIT_CODE%
)

endlocal
