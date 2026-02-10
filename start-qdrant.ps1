# Script PowerShell para iniciar o Qdrant via Docker

Write-Host "🚀 Iniciando Qdrant Vector Database..." -ForegroundColor Cyan
Write-Host ""

# Verificar se Docker está rodando
try {
    docker info | Out-Null
} catch {
    Write-Host "❌ Docker não está rodando. Por favor, inicie o Docker Desktop." -ForegroundColor Red
    exit 1
}

# Verificar se já existe container
$existing = docker ps -a --filter "name=jarvis-qdrant" --format "{{.Names}}"
if ($existing -eq "jarvis-qdrant") {
    Write-Host "⚠️  Container 'jarvis-qdrant' já existe." -ForegroundColor Yellow
    Write-Host ""
    $running = docker ps --filter "name=jarvis-qdrant" --format "{{.Names}}"
    if ($running -eq "jarvis-qdrant") {
        Write-Host "✅ Qdrant já está rodando!" -ForegroundColor Green
    } else {
        Write-Host "▶️  Iniciando container existente..." -ForegroundColor Cyan
        docker start jarvis-qdrant
    }
} else {
    # Criar e iniciar novo container
    Write-Host "📦 Criando novo container Qdrant..." -ForegroundColor Cyan
    $currentDir = Get-Location
    docker run -d `
        --name jarvis-qdrant `
        -p 6333:6333 `
        -p 6334:6334 `
        -v "${currentDir}\qdrant_storage:/qdrant/storage" `
        qdrant/qdrant:latest
}

Write-Host ""
Write-Host "✅ Qdrant iniciado com sucesso!" -ForegroundColor Green
Write-Host "📊 Dashboard: http://localhost:6333/dashboard" -ForegroundColor Cyan
Write-Host "🔌 API: http://localhost:6333" -ForegroundColor Cyan
Write-Host ""
Write-Host "Para parar: docker stop jarvis-qdrant" -ForegroundColor Yellow
Write-Host "Para remover: docker rm jarvis-qdrant" -ForegroundColor Yellow
Write-Host ""

# Testar conexão
Write-Host "🔍 Testando conexão..." -ForegroundColor Cyan
Start-Sleep -Seconds 2

try {
    $response = Invoke-RestMethod -Uri "http://localhost:6333/collections" -Method Get
    Write-Host "✅ Qdrant está respondendo corretamente!" -ForegroundColor Green
    Write-Host ""
    Write-Host "Próximo passo: Indexar o projeto" -ForegroundColor Cyan
    Write-Host "  ./index-project.sh" -ForegroundColor White
    Write-Host ""
} catch {
    Write-Host "⚠️  Qdrant ainda está inicializando. Aguarde alguns segundos." -ForegroundColor Yellow
}
