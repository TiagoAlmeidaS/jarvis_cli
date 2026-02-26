# Script para executar Jarvis CLI com estratégia Free
# Uso: .\scripts\run-jarvis-free.ps1

param(
    [string]$Model = "openrouter/free",
    [string]$Provider = "openrouter",
    [string]$Prompt = ""
)

Write-Host "============================================================" -ForegroundColor Cyan
Write-Host "  Jarvis CLI - Modo Free" -ForegroundColor Cyan
Write-Host "============================================================" -ForegroundColor Cyan
Write-Host ""

# Verificar se estamos no diretório correto
if (-not (Test-Path "jarvis-rs")) {
    Write-Host "ERRO: Execute este script da raiz do projeto (jarvis_cli)" -ForegroundColor Red
    exit 1
}

# Verificar variáveis de ambiente
if ($Provider -eq "openrouter" -and -not $env:OPENROUTER_API_KEY) {
    Write-Host "AVISO: OPENROUTER_API_KEY não configurada!" -ForegroundColor Yellow
    Write-Host "Configure com: `$env:OPENROUTER_API_KEY = 'sua-chave'" -ForegroundColor Yellow
    Write-Host "Obtenha em: https://openrouter.ai/keys" -ForegroundColor Yellow
    Write-Host ""
}

if ($Provider -eq "google" -and -not $env:GOOGLE_API_KEY) {
    Write-Host "AVISO: GOOGLE_API_KEY não configurada!" -ForegroundColor Yellow
    Write-Host "Configure com: `$env:GOOGLE_API_KEY = 'sua-chave'" -ForegroundColor Yellow
    Write-Host "Obtenha em: https://ai.google.dev" -ForegroundColor Yellow
    Write-Host ""
}

# Navegar para jarvis-rs
Set-Location jarvis-rs

# Verificar se o binário existe
$jarvisExe = "target\release\jarvis.exe"
if (-not (Test-Path $jarvisExe)) {
    $jarvisExe = "target\debug\jarvis.exe"
    if (-not (Test-Path $jarvisExe)) {
        Write-Host "ERRO: Jarvis não compilado. Execute primeiro:" -ForegroundColor Red
        Write-Host "  cargo build --package jarvis-cli --release" -ForegroundColor Yellow
        Set-Location ..
        exit 1
    }
}

Write-Host "Provider: $Provider" -ForegroundColor Green
Write-Host "Model: $Model" -ForegroundColor Green
Write-Host ""

# Executar Jarvis
if ($Prompt) {
    Write-Host "Executando comando não-interativo..." -ForegroundColor Cyan
    Write-Host ""
    & ".\$jarvisExe" exec -c "model_provider=$Provider" -m $Model $Prompt
} else {
    Write-Host "Iniciando chat interativo..." -ForegroundColor Cyan
    Write-Host "Pressione Ctrl+C para sair" -ForegroundColor Gray
    Write-Host ""
    & ".\$jarvisExe" chat -c "model_provider=$Provider" -m $Model
}

Set-Location ..
