# Script para testar Jarvis Web API localmente (sem Docker)
# Windows PowerShell

$ErrorActionPreference = "Stop"

# Obter o diretorio do script
$scriptPath = $MyInvocation.MyCommand.Path
if (-not $scriptPath) {
    # Se executado diretamente, tentar encontrar o script
    $scriptPath = $PSCommandPath
}
if (-not $scriptPath) {
    Write-Host "ERRO - Nao foi possivel determinar o caminho do script" -ForegroundColor Red
    Write-Host "Execute o script da raiz do projeto: .\scripts\test-local-web-api.ps1" -ForegroundColor Yellow
    exit 1
}

$scriptDir = Split-Path -Parent $scriptPath
# O projeto esta um nivel acima de scripts/
$projectRoot = Split-Path -Parent $scriptDir

# Verificar se estamos no diretorio correto
if (-not (Test-Path (Join-Path $projectRoot "jarvis-rs"))) {
    Write-Host "ERRO - Diretorio do projeto nao encontrado: $projectRoot" -ForegroundColor Red
    Write-Host "Execute o script da raiz do projeto (jarvis_cli)" -ForegroundColor Yellow
    exit 1
}

# Mudar para o diretorio raiz do projeto
Set-Location $projectRoot
Write-Host "Diretorio do projeto: $projectRoot" -ForegroundColor Cyan
Write-Host ""

Write-Host "Testando Jarvis Web API Localmente (sem Docker)" -ForegroundColor Cyan
Write-Host ""

# 1. Verificar se Rust esta instalado
Write-Host "Verificando Rust..." -ForegroundColor Yellow
try {
    $rustVersion = cargo --version 2>&1
    Write-Host "OK - Rust encontrado: $rustVersion" -ForegroundColor Green
} catch {
    Write-Host "ERRO - Rust nao encontrado!" -ForegroundColor Red
    Write-Host "   Instale Rust: https://rustup.rs/" -ForegroundColor Yellow
    exit 1
}
Write-Host ""

# 2. Compilar o projeto
Write-Host "Compilando jarvis-web-api..." -ForegroundColor Yellow
Set-Location jarvis-rs
cargo build --package jarvis-web-api --release
if ($LASTEXITCODE -ne 0) {
    Write-Host "ERRO - Falha na compilacao!" -ForegroundColor Red
    exit 1
}
Write-Host "OK - Compilacao concluida!" -ForegroundColor Green
Write-Host ""

# 3. Configurar JARVIS_HOME e config.toml
Write-Host "Configurando ambiente..." -ForegroundColor Yellow
$jarvisHome = "$env:USERPROFILE\.jarvis"
if (-not (Test-Path $jarvisHome)) {
    New-Item -ItemType Directory -Path $jarvisHome | Out-Null
    Write-Host "OK - Diretorio criado: $jarvisHome" -ForegroundColor Green
}

$configPath = Join-Path $jarvisHome "config.toml"
if (-not (Test-Path $configPath)) {
    Write-Host "Criando config.toml..." -ForegroundColor Yellow
    $apiKey = -join ((65..90) + (97..122) + (48..57) | Get-Random -Count 32 | ForEach-Object {[char]$_})
    $configContent = @"
[api]
api_key = "$apiKey"
port = 3000
bind_address = "127.0.0.1"
enable_cors = true
"@
    Set-Content -Path $configPath -Value $configContent -Encoding UTF8
    Write-Host "OK - config.toml criado!" -ForegroundColor Green
    Write-Host "API Key gerada: $apiKey" -ForegroundColor Cyan
    Write-Host ""
} else {
    Write-Host "OK - config.toml ja existe: $configPath" -ForegroundColor Green
    # Tentar extrair API key existente
    $apiKeyLine = Get-Content $configPath | Select-String "api_key"
    if ($apiKeyLine) {
        $apiKey = ($apiKeyLine -split '=')[1].Trim().Trim('"')
        Write-Host "API Key: $apiKey" -ForegroundColor Cyan
    }
    Write-Host ""
}

# 4. Definir variaveis de ambiente
$env:JARVIS_HOME = $jarvisHome
$env:RUST_LOG = "info,jarvis_web_api=debug"

# 5. Caminho do binario
$binaryPath = Join-Path $projectRoot "jarvis-rs\target\release\jarvis-web-api.exe"
if (-not (Test-Path $binaryPath)) {
    Write-Host "ERRO - Binario nao encontrado: $binaryPath" -ForegroundColor Red
    exit 1
}

Write-Host "Iniciando servidor..." -ForegroundColor Yellow
Write-Host ""
Write-Host "============================================================" -ForegroundColor Cyan
Write-Host "  Jarvis Web API - Servidor Local" -ForegroundColor White
Write-Host "============================================================" -ForegroundColor Cyan
Write-Host ""
Write-Host "URL: http://localhost:3000" -ForegroundColor Green
Write-Host "Health: http://localhost:3000/api/health" -ForegroundColor Green
Write-Host "API Key: $apiKey" -ForegroundColor Yellow
Write-Host ""
Write-Host "Dica: Abra outro terminal para testar:" -ForegroundColor Cyan
Write-Host "   curl http://localhost:3000/api/health" -ForegroundColor White
Write-Host ""
Write-Host "Pressione Ctrl+C para parar o servidor" -ForegroundColor Yellow
Write-Host ""
Write-Host "============================================================" -ForegroundColor Cyan
Write-Host ""

# 6. Executar o servidor
try {
    & $binaryPath
} catch {
    Write-Host ""
    Write-Host "ERRO - Erro ao executar servidor: $_" -ForegroundColor Red
    exit 1
}
