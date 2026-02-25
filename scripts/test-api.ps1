# Script para testar a API apos iniciar o servidor
# Windows PowerShell

$ErrorActionPreference = "Stop"

Write-Host "Testando Jarvis Web API" -ForegroundColor Cyan
Write-Host ""

# Ler API key do config.toml
$jarvisHome = "$env:USERPROFILE\.jarvis"
$configPath = Join-Path $jarvisHome "config.toml"

if (-not (Test-Path $configPath)) {
    Write-Host "ERRO - config.toml nao encontrado!" -ForegroundColor Red
    Write-Host "   Execute primeiro: .\scripts\test-local-web-api.ps1" -ForegroundColor Yellow
    exit 1
}

$apiKeyLine = Get-Content $configPath | Select-String "api_key"
if (-not $apiKeyLine) {
    Write-Host "ERRO - API key nao encontrada no config.toml!" -ForegroundColor Red
    exit 1
}

$apiKey = ($apiKeyLine -split '=')[1].Trim().Trim('"')
Write-Host "API Key: $apiKey" -ForegroundColor Yellow
Write-Host ""

# Teste 1: Health Check
Write-Host "1. Testando Health Check..." -ForegroundColor Yellow
try {
    $response = Invoke-RestMethod -Uri "http://localhost:3000/api/health" -Method Get
    Write-Host "OK - Health Check passou!" -ForegroundColor Green
    Write-Host "   Status: $($response.status)" -ForegroundColor White
    Write-Host "   Version: $($response.version)" -ForegroundColor White
} catch {
    Write-Host "ERRO - Health Check falhou!" -ForegroundColor Red
    Write-Host "   Certifique-se de que o servidor esta rodando" -ForegroundColor Yellow
    Write-Host "   Erro: $_" -ForegroundColor Red
}
Write-Host ""

# Teste 2: Config Endpoint
Write-Host "2. Testando Config Endpoint..." -ForegroundColor Yellow
try {
    $headers = @{
        "Authorization" = "Bearer $apiKey"
    }
    $response = Invoke-RestMethod -Uri "http://localhost:3000/api/config" -Method Get -Headers $headers
    Write-Host "OK - Config passou!" -ForegroundColor Green
    Write-Host "   Model Provider: $($response.model_provider)" -ForegroundColor White
    Write-Host "   Port: $($response.port)" -ForegroundColor White
} catch {
    Write-Host "ERRO - Config falhou!" -ForegroundColor Red
    Write-Host "   Erro: $_" -ForegroundColor Red
}
Write-Host ""

# Teste 3: Chat Endpoint
Write-Host "3. Testando Chat Endpoint..." -ForegroundColor Yellow
try {
    $headers = @{
        "Authorization" = "Bearer $apiKey"
        "Content-Type" = "application/json"
    }
    $body = @{
        prompt = "Ola, Jarvis! Este e um teste."
    } | ConvertTo-Json
    
    $response = Invoke-RestMethod -Uri "http://localhost:3000/api/chat" -Method Post -Headers $headers -Body $body
    Write-Host "OK - Chat passou!" -ForegroundColor Green
    Write-Host "   Thread ID: $($response.thread_id)" -ForegroundColor White
    $replyPreview = if ($response.reply.Length -gt 100) { $response.reply.Substring(0, 100) + "..." } else { $response.reply }
    Write-Host "   Reply: $replyPreview" -ForegroundColor White
} catch {
    Write-Host "ERRO - Chat falhou!" -ForegroundColor Red
    Write-Host "   Erro: $_" -ForegroundColor Red
}
Write-Host ""

# Teste 4: Interface Web
Write-Host "4. Testando Interface Web..." -ForegroundColor Yellow
try {
    $response = Invoke-WebRequest -Uri "http://localhost:3000/" -Method Get
    if ($response.StatusCode -eq 200) {
        Write-Host "OK - Interface Web funcionando!" -ForegroundColor Green
        Write-Host "   Abra no navegador: http://localhost:3000" -ForegroundColor Cyan
    }
} catch {
    Write-Host "ERRO - Interface Web falhou!" -ForegroundColor Red
    Write-Host "   Erro: $_" -ForegroundColor Red
}
Write-Host ""

Write-Host "============================================================" -ForegroundColor Cyan
Write-Host "Testes concluidos!" -ForegroundColor Green
Write-Host ""
Write-Host "Proximos passos:" -ForegroundColor Cyan
Write-Host "   - Acesse a interface web: http://localhost:3000" -ForegroundColor White
Write-Host "   - Use a API Key para autenticacao: $apiKey" -ForegroundColor White
Write-Host "============================================================" -ForegroundColor Cyan
