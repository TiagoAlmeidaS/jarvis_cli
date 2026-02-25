# Script de teste local para Windows PowerShell
# Testa a API Web localmente antes do deploy

Write-Host "🧪 Testando Jarvis Web API Localmente..." -ForegroundColor Cyan

# 1. Compilar
Write-Host "`n📦 Compilando..." -ForegroundColor Yellow
cd ..
cargo build --package jarvis-web-api --release
if ($LASTEXITCODE -ne 0) {
    Write-Host "❌ Erro na compilação!" -ForegroundColor Red
    exit 1
}
Write-Host "✅ Compilação concluída!" -ForegroundColor Green

# 2. Verificar se config.toml existe
$configPath = "$env:USERPROFILE\.jarvis\config.toml"
if (-not (Test-Path $configPath)) {
    Write-Host "`n⚠️  config.toml não encontrado em $configPath" -ForegroundColor Yellow
    Write-Host "Criando config.toml básico..." -ForegroundColor Yellow
    
    $jarvisDir = "$env:USERPROFILE\.jarvis"
    if (-not (Test-Path $jarvisDir)) {
        New-Item -ItemType Directory -Path $jarvisDir -Force | Out-Null
    }
    
    # Gerar API key
    $apiKey = -join ((65..90) + (97..122) + (48..57) | Get-Random -Count 32 | ForEach-Object {[char]$_})
    
    $configContent = @"
[api]
api_key = "$apiKey"
port = 3000
bind_address = "127.0.0.1"
enable_cors = true
"@
    
    Set-Content -Path $configPath -Value $configContent
    Write-Host "✅ config.toml criado!" -ForegroundColor Green
    Write-Host "🔑 API Key gerada: $apiKey" -ForegroundColor Cyan
    Write-Host "⚠️  Salve esta API key em local seguro!" -ForegroundColor Yellow
} else {
    Write-Host "✅ config.toml encontrado" -ForegroundColor Green
}

# 3. Iniciar servidor em background
Write-Host "`n🚀 Iniciando servidor..." -ForegroundColor Yellow
$serverProcess = Start-Process -FilePath ".\target\release\jarvis-web-api.exe" -PassThru -NoNewWindow

# Aguardar servidor iniciar
Write-Host "⏳ Aguardando servidor iniciar..." -ForegroundColor Yellow
Start-Sleep -Seconds 3

# 4. Testar health check
Write-Host "`n🏥 Testando health check..." -ForegroundColor Yellow
try {
    $healthResponse = Invoke-WebRequest -Uri "http://localhost:3000/api/health" -UseBasicParsing -TimeoutSec 5
    if ($healthResponse.StatusCode -eq 200) {
        Write-Host "✅ Health check OK!" -ForegroundColor Green
        Write-Host "   Resposta: $($healthResponse.Content)" -ForegroundColor Gray
    }
} catch {
    Write-Host "❌ Health check falhou: $_" -ForegroundColor Red
    Stop-Process -Id $serverProcess.Id -Force -ErrorAction SilentlyContinue
    exit 1
}

# 5. Mostrar informações
Write-Host "`n✅ Servidor rodando!" -ForegroundColor Green
Write-Host "📍 URL: http://localhost:3000" -ForegroundColor Cyan
Write-Host "🌐 Interface Web: http://localhost:3000/" -ForegroundColor Cyan
Write-Host "📡 API Health: http://localhost:3000/api/health" -ForegroundColor Cyan
Write-Host "`n💡 Para parar o servidor, pressione Ctrl+C ou feche esta janela" -ForegroundColor Yellow

# Manter processo rodando
try {
    Wait-Process -Id $serverProcess.Id
} catch {
    Write-Host "`n🛑 Servidor parado" -ForegroundColor Yellow
}
