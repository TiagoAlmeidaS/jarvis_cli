@echo off
REM ============================================================================
REM Run Script for Jarvis CLI with Ollama Integration (Windows)
REM ============================================================================
REM Usage: run-jarvis.bat [model]
REM   model: llama3.2:3b (default), gemma2:2b, qwen2.5:7b, etc.
REM
REM Examples:
REM   run-jarvis.bat
REM   run-jarvis.bat gemma2:2b
REM ============================================================================

setlocal enabledelayedexpansion

REM Configurações
set "DEFAULT_MODEL=llama3.2:3b"
set "OLLAMA_VPS=http://100.98.213.86:11434/v1"

if "%1"=="" (
    set "MODEL=%DEFAULT_MODEL%"
) else (
    set "MODEL=%1"
)

echo ========================================
echo 🚀 Jarvis CLI + Ollama VPS
echo ========================================
echo.

REM Verificar se o binário existe
set "BINARY_DEBUG=jarvis-rs\target\debug\jarvis.exe"
set "BINARY_RELEASE=jarvis-rs\target\release\jarvis.exe"

if exist "%BINARY_RELEASE%" (
    set "BINARY=%BINARY_RELEASE%"
    set "BUILD_TYPE=release"
) else if exist "%BINARY_DEBUG%" (
    set "BINARY=%BINARY_DEBUG%"
    set "BUILD_TYPE=debug"
) else (
    echo ❌ Erro: Binário não encontrado!
    echo.
    echo Execute primeiro:
    echo   build-jarvis.bat
    echo.
    pause
    exit /b 1
)

echo ✓ Binário encontrado: %BINARY% (%BUILD_TYPE%)
echo ✓ Ollama VPS: %OLLAMA_VPS%
echo ✓ Modelo: %MODEL%
echo.

REM Configurar variável de ambiente
set "OLLAMA_BASE_URL=%OLLAMA_VPS%"

echo ========================================
echo 🎯 Iniciando Jarvis...
echo ========================================
echo.
echo 🎨 Abrindo modo chat interativo...
echo.
echo Dicas:
echo   • Ctrl+C para sair
echo   • Primeira resposta pode demorar ~20-30s (warm-up)
echo   • Respostas seguintes serão rápidas
echo.
echo ========================================
echo.

REM Executar Jarvis
"%BINARY%" -c "model_provider=\"ollama\"" -m "%MODEL%"

if errorlevel 1 (
    echo.
    echo ❌ Erro ao executar Jarvis
    pause
    exit /b 1
)

endlocal
