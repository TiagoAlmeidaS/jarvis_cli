# Script para iniciar o Jarvis Web API com Docker Compose (PowerShell)

$ErrorActionPreference = "Stop"

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Definition
$projectRoot = Split-Path -Parent $scriptDir

Set-Location $projectRoot

Write-Host "🚀 Iniciando Jarvis Web API com Docker Compose..." -ForegroundColor Cyan
Write-Host ""

# Verificar se .env existe
if (-not (Test-Path .env)) {
    Write-Host "⚠️  Arquivo .env não encontrado!" -ForegroundColor Yellow
    Write-Host "📝 Criando .env a partir de .env.example..." -ForegroundColor Cyan
    
    if (Test-Path .env.example) {
        Copy-Item .env.example .env
        
        # Gerar API key automaticamente
        $apiKey = -join ((65..90) + (97..122) + (48..57) | Get-Random -Count 32 | ForEach-Object {[char]$_})
        (Get-Content .env) -replace 'your-api-key-here', $apiKey | Set-Content .env
        
        Write-Host "✅ Arquivo .env criado!" -ForegroundColor Green
        Write-Host "🔑 API Key gerada: $apiKey" -ForegroundColor Green
        Write-Host ""
    } else {
        Write-Host "❌ Arquivo .env.example não encontrado!" -ForegroundColor Red
        Write-Host "📝 Criando .env básico..." -ForegroundColor Cyan
        
        $apiKey = -join ((65..90) + (97..122) + (48..57) | Get-Random -Count 32 | ForEach-Object {[char]$_})
        @"
JARVIS_API_KEY=$apiKey
WEB_API_PORT=3000
RUST_LOG=info
"@ | Out-File -FilePath .env -Encoding UTF8
        
        Write-Host "✅ Arquivo .env criado!" -ForegroundColor Green
    }
    Write-Host ""
}

# Verificar se Docker está rodando
try {
    docker info | Out-Null
} catch {
    Write-Host "❌ Docker não está rodando!" -ForegroundColor Red
    Write-Host "   Por favor, inicie o Docker Desktop e tente novamente." -ForegroundColor Yellow
    exit 1
}

# Verificar se docker-compose está disponível
$composeCmd = "docker compose"
if (-not (docker compose version 2>$null)) {
    if (Get-Command docker-compose -ErrorAction SilentlyContinue) {
        $composeCmd = "docker-compose"
    } else {
        Write-Host "❌ docker-compose não encontrado!" -ForegroundColor Red
        exit 1
    }
}

# Subir os serviços
Write-Host "🐳 Subindo containers..." -ForegroundColor Cyan
& $composeCmd.Split(' ') -f docker-compose.web-api.yml up -d

Write-Host ""
Write-Host "⏳ Aguardando serviço iniciar..." -ForegroundColor Cyan
Start-Sleep -Seconds 5

# Verificar saúde
try {
    $healthCheck = docker exec jarvis-web-api wget --quiet --tries=1 --spider http://localhost:3000/api/health 2>&1
    if ($LASTEXITCODE -eq 0) {
        Write-Host ""
        Write-Host "✅ Jarvis Web API está rodando!" -ForegroundColor Green
        Write-Host ""
        Write-Host "📊 Informações:" -ForegroundColor Cyan
        Write-Host "   - URL: http://localhost:3000" -ForegroundColor White
        Write-Host "   - Health: http://localhost:3000/api/health" -ForegroundColor White
        Write-Host "   - Logs: $composeCmd -f docker-compose.web-api.yml logs -f" -ForegroundColor White
        Write-Host "   - Parar: $composeCmd -f docker-compose.web-api.yml down" -ForegroundColor White
        Write-Host ""
        
        # Mostrar API key do .env
        if (Test-Path .env) {
            $apiKeyLine = Get-Content .env | Select-String "JARVIS_API_KEY"
            if ($apiKeyLine) {
                $apiKey = ($apiKeyLine -split '=')[1]
                Write-Host "🔑 API Key: $apiKey" -ForegroundColor Yellow
                Write-Host ""
            }
        }
    } else {
        Write-Host ""
        Write-Host "⚠️  Serviço iniciado, mas health check ainda não passou." -ForegroundColor Yellow
        Write-Host "   Verifique os logs: $composeCmd -f docker-compose.web-api.yml logs -f" -ForegroundColor Yellow
        Write-Host ""
    }
} catch {
    Write-Host ""
    Write-Host "⚠️  Não foi possível verificar o health check." -ForegroundColor Yellow
    Write-Host "   Verifique os logs: $composeCmd -f docker-compose.web-api.yml logs -f" -ForegroundColor Yellow
    Write-Host ""
}
