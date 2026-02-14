@echo off
REM ============================================================================
REM Build Script for Jarvis CLI (Windows)
REM ============================================================================
REM Usage: build-jarvis.bat [release|debug]
REM Default: debug
REM ============================================================================

setlocal enabledelayedexpansion

REM Navegar para a raiz do projeto (pai do diretório scripts/)
cd /d "%~dp0.."

set "BUILD_TYPE=%1"
if "%BUILD_TYPE%"=="" set "BUILD_TYPE=debug"

echo ========================================
echo 🔨 Building Jarvis CLI
echo ========================================
echo.

REM Verificar se estamos no diretório correto
if not exist "jarvis-rs" (
    echo ❌ Erro: Diretório jarvis-rs não encontrado!
    echo Verifique que o script está dentro da pasta scripts/ do projeto
    pause
    exit /b 1
)

echo 📂 Diretório: %CD%
echo 🔧 Tipo de build: %BUILD_TYPE%
echo.

REM Navegar para o diretório do Rust
cd jarvis-rs

REM Executar build
echo 🔨 Compilando...
echo.

set START_TIME=%TIME%

if "%BUILD_TYPE%"=="release" (
    echo Building RELEASE ^(otimizado, pode levar tempo^)...
    cargo build --release --bin jarvis
    set "BINARY_PATH=target\release\jarvis.exe"
) else (
    echo Building DEBUG ^(mais rapido, sem otimizacoes^)...
    cargo build --bin jarvis
    set "BINARY_PATH=target\debug\jarvis.exe"
)

if errorlevel 1 (
    echo.
    echo ========================================
    echo ❌ Build falhou!
    echo ========================================
    echo.
    cd ..
    pause
    exit /b 1
)

set END_TIME=%TIME%

REM Voltar ao diretório raiz
cd ..

REM Verificar se o binário foi criado
if exist "jarvis-rs\%BINARY_PATH%" (
    for %%A in ("jarvis-rs\%BINARY_PATH%") do set BINARY_SIZE=%%~zA
    set /a "BINARY_SIZE_MB=!BINARY_SIZE! / 1048576"

    echo.
    echo ========================================
    echo ✅ Build concluído com sucesso!
    echo ========================================
    echo.
    echo 📦 Binário criado: jarvis-rs\%BINARY_PATH%
    echo 💾 Tamanho: !BINARY_SIZE_MB! MB
    echo.
    echo Para executar:
    echo   scripts\run-jarvis.bat
    echo.
    echo Ou manualmente:
    echo   set OLLAMA_BASE_URL=http://100.98.213.86:11434/v1
    echo   jarvis-rs\%BINARY_PATH% -c "model_provider=\"ollama\"" -m llama3.2:3b
    echo.
) else (
    echo.
    echo ========================================
    echo ❌ Binário não foi criado!
    echo ========================================
    echo.
)

pause
endlocal
