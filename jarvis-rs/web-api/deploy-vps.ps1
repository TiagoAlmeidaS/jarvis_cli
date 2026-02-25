# Script de Deploy para VPS - Windows PowerShell
# Facilita o processo de build e transferência

param(
    [Parameter(Mandatory=$true)]
    [string]$VpsUser,
    
    [Parameter(Mandatory=$true)]
    [string]$VpsIp,
    
    [Parameter(Mandatory=$false)]
    [string]$VpsPath = "/opt/jarvis",
    
    [Parameter(Mandatory=$false)]
    [switch]$SkipBuild
)

$ErrorActionPreference = "Stop"

Write-Host "🚀 Deploy Jarvis Web API para VPS" -ForegroundColor Cyan
Write-Host "====================================" -ForegroundColor Cyan

# 1. Verificar pré-requisitos
Write-Host "`n📋 Verificando pré-requisitos..." -ForegroundColor Yellow

# Verificar Rust
try {
    $rustVersion = rustc --version
    Write-Host "✅ Rust encontrado: $rustVersion" -ForegroundColor Green
} catch {
    Write-Host "❌ Rust não encontrado! Instale em: https://rustup.rs" -ForegroundColor Red
    exit 1
}

# Verificar target Linux
Write-Host "`n🔧 Verificando target Linux..." -ForegroundColor Yellow
$targetInstalled = rustup target list --installed | Select-String "x86_64-unknown-linux-musl"
if (-not $targetInstalled) {
    Write-Host "📦 Instalando target Linux..." -ForegroundColor Yellow
    rustup target add x86_64-unknown-linux-musl
    if ($LASTEXITCODE -ne 0) {
        Write-Host "❌ Erro ao instalar target!" -ForegroundColor Red
        exit 1
    }
}
Write-Host "✅ Target Linux instalado" -ForegroundColor Green

# 2. Compilar
if (-not $SkipBuild) {
    Write-Host "`n🔨 Compilando para Linux..." -ForegroundColor Yellow
    cd ..
    cargo build --package jarvis-web-api --release --target x86_64-unknown-linux-musl
    if ($LASTEXITCODE -ne 0) {
        Write-Host "❌ Erro na compilação!" -ForegroundColor Red
        exit 1
    }
    Write-Host "✅ Compilação concluída!" -ForegroundColor Green
    cd web-api
} else {
    Write-Host "`n⏭️  Pulando compilação (--SkipBuild)" -ForegroundColor Yellow
}

# 3. Verificar binário
$binaryPath = "..\target\x86_64-unknown-linux-musl\release\jarvis-web-api"
if (-not (Test-Path $binaryPath)) {
    Write-Host "❌ Binário não encontrado em: $binaryPath" -ForegroundColor Red
    exit 1
}
Write-Host "✅ Binário encontrado" -ForegroundColor Green

# 4. Gerar API Key (se não existir)
Write-Host "`n🔑 Verificando API Key..." -ForegroundColor Yellow
$apiKeyFile = "$env:USERPROFILE\.jarvis-api-key.txt"
if (-not (Test-Path $apiKeyFile)) {
    $apiKey = -join ((65..90) + (97..122) + (48..57) | Get-Random -Count 32 | ForEach-Object {[char]$_})
    Set-Content -Path $apiKeyFile -Value $apiKey
    Write-Host "✅ Nova API Key gerada: $apiKey" -ForegroundColor Green
    Write-Host "⚠️  Salve esta key em local seguro!" -ForegroundColor Yellow
} else {
    $apiKey = Get-Content $apiKeyFile
    Write-Host "✅ API Key existente encontrada" -ForegroundColor Green
}

# 5. Transferir binário
Write-Host "`n📤 Transferindo binário para VPS..." -ForegroundColor Yellow
$remoteBinary = "${VpsUser}@${VpsIp}:${VpsPath}/jarvis-web-api"
scp $binaryPath $remoteBinary
if ($LASTEXITCODE -ne 0) {
    Write-Host "❌ Erro ao transferir binário!" -ForegroundColor Red
    exit 1
}
Write-Host "✅ Binário transferido!" -ForegroundColor Green

# 6. Transferir arquivos estáticos
Write-Host "`n📤 Transferindo arquivos estáticos..." -ForegroundColor Yellow
if (Test-Path "static") {
    $remoteStatic = "${VpsUser}@${VpsIp}:${VpsPath}/static"
    scp -r static $remoteStatic
    if ($LASTEXITCODE -ne 0) {
        Write-Host "⚠️  Aviso: Erro ao transferir estáticos (pode ser normal)" -ForegroundColor Yellow
    } else {
        Write-Host "✅ Arquivos estáticos transferidos!" -ForegroundColor Green
    }
}

# 7. Instruções finais
Write-Host "`n✅ Deploy concluído!" -ForegroundColor Green
Write-Host "`n📋 Próximos passos na VPS:" -ForegroundColor Cyan
Write-Host "1. Conectar: ssh ${VpsUser}@${VpsIp}" -ForegroundColor White
Write-Host "2. Tornar executável: chmod +x ${VpsPath}/jarvis-web-api" -ForegroundColor White
Write-Host "3. Criar config.toml em ${VpsPath}/.jarvis/config.toml" -ForegroundColor White
Write-Host "   Com API Key: $apiKey" -ForegroundColor Gray
Write-Host "4. Criar systemd service (veja DEPLOY_VPS.md)" -ForegroundColor White
Write-Host "5. Iniciar: sudo systemctl start jarvis-web-api" -ForegroundColor White
Write-Host "`n📚 Veja DEPLOY_VPS.md para instruções completas" -ForegroundColor Cyan
