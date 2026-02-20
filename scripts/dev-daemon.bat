@echo off
REM ============================================================================
REM Dev Script for Jarvis Daemon (Windows)
REM ============================================================================
REM Compiles and runs the Jarvis Daemon (background automation) using cargo run.
REM The daemon executes scheduled pipelines (SEO blog, YouTube shorts, etc.)
REM autonomously, without human interaction.
REM
REM Usage: dev-daemon.bat [options]
REM
REM Options:
REM   --release          Build in release mode
REM   --tick N           Scheduler tick interval in seconds (default: 60)
REM   --concurrent N     Max concurrent jobs (default: 3)
REM   --db PATH          Custom database path (default: ~/.jarvis/daemon.db)
REM   --debug            Enable debug-level logging
REM
REM Prerequisites:
REM   - Set OPENROUTER_API_KEY in .env (or the env var for your chosen provider)
REM   - Create at least one pipeline via: jarvis daemon pipeline create ...
REM
REM Examples:
REM   dev-daemon.bat                          (default: tick 60s, 3 concurrent)
REM   dev-daemon.bat --tick 3600              (check pipelines every hour)
REM   dev-daemon.bat --tick 10 --debug        (fast ticks + debug logging)
REM   dev-daemon.bat --release --tick 300     (release build, 5 min ticks)
REM ============================================================================

setlocal enabledelayedexpansion

REM Navigate to project root (parent of scripts/)
cd /d "%~dp0.."

REM Defaults
set "CARGO_FLAGS="
set "TICK_INTERVAL=60"
set "MAX_CONCURRENT=3"
set "DB_PATH="
set "LOG_LEVEL=jarvis_daemon=info"

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
if "%1"=="--tick" (
    set "TICK_INTERVAL=%2"
    shift
    shift
    goto :parse_args
)
if "%1"=="--concurrent" (
    set "MAX_CONCURRENT=%2"
    shift
    shift
    goto :parse_args
)
if "%1"=="--db" (
    set "DB_PATH=%2"
    shift
    shift
    goto :parse_args
)
if "%1"=="--debug" (
    set "LOG_LEVEL=jarvis_daemon=debug"
    shift
    goto :parse_args
)
echo Argumento desconhecido: %1
shift
goto :parse_args
:done_args

REM Set log level via env var
set "RUST_LOG=%LOG_LEVEL%"

echo ========================================
echo  Jarvis Daemon - Dev Mode
echo ========================================
echo.
echo  Tick interval:   %TICK_INTERVAL%s
echo  Max concurrent:  %MAX_CONCURRENT%
if defined DB_PATH (
    echo  Database:        %DB_PATH%
) else (
    echo  Database:        ~/.jarvis/daemon.db [default]
)
echo  Log level:       %LOG_LEVEL%
if defined CARGO_FLAGS (
    echo  Build:           release
) else (
    echo  Build:           debug [incremental]
)
echo.

REM Check for API keys
if defined OPENROUTER_API_KEY (
    echo  OpenRouter API:  configurada
) else (
    echo  OpenRouter API:  nao definida
)
if defined GOOGLE_API_KEY (
    echo  Google API:      configurada (Gemini)
) else (
    echo  Google API:      nao definida
)
if not defined OPENROUTER_API_KEY if not defined GOOGLE_API_KEY if not defined OPENAI_API_KEY (
    echo.
    echo  [AVISO] Nenhuma API key detectada (OPENROUTER_API_KEY, GOOGLE_API_KEY, OPENAI_API_KEY).
    echo          O daemon usara a env var do provider configurado no pipeline.
    echo          Defina em jarvis-rs/.env ou como variavel de ambiente.
)
echo.
echo  Compilando e executando...
echo  Pressione Ctrl+C para parar o daemon.
echo ========================================
echo.

REM Build daemon args
set "DAEMON_ARGS=--tick-interval-sec %TICK_INTERVAL% --max-concurrent %MAX_CONCURRENT%"
if defined DB_PATH (
    set "DAEMON_ARGS=!DAEMON_ARGS! --db-path "%DB_PATH%""
)

REM Use cargo run for incremental compilation + execution
cd jarvis-rs
cargo run %CARGO_FLAGS% --bin jarvis-daemon -- %DAEMON_ARGS%
set "EXIT_CODE=%ERRORLEVEL%"
cd ..

if %EXIT_CODE% neq 0 (
    echo.
    echo  Daemon encerrou com codigo %EXIT_CODE%
    pause
    exit /b %EXIT_CODE%
)

endlocal
