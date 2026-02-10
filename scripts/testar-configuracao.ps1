# Script simples para testar a configuração do Jarvis CLI
# Execute: .\testar-configuracao.ps1

Write-Host "`n=== Teste de Configuração do Jarvis CLI ===" -ForegroundColor Cyan
Write-Host ""

$configFile = "$env:USERPROFILE\.jarvis\config.toml"

# 1. Verificar arquivo config.toml
Write-Host "1. Verificando arquivo config.toml..." -ForegroundColor Yellow
if (Test-Path $configFile) {
    Write-Host "   ✅ Arquivo encontrado: $configFile" -ForegroundColor Green
    $size = (Get-Item $configFile).Length
    Write-Host "   📄 Tamanho: $size bytes" -ForegroundColor Gray
} else {
    Write-Host "   ❌ Arquivo não encontrado!" -ForegroundColor Red
    Write-Host "   Execute: Copy-Item '.\config.toml.example' '$configFile'" -ForegroundColor Yellow
    exit 1
}

# 2. Verificar variáveis de ambiente
Write-Host "`n2. Verificando variáveis de ambiente..." -ForegroundColor Yellow

$vars = @("DATABRICKS_API_KEY", "OPENAI_API_KEY", "OPENROUTER_API_KEY")
$missing = @()

foreach ($var in $vars) {
    $value = [System.Environment]::GetEnvironmentVariable($var, "User")
    if ([string]::IsNullOrEmpty($value)) {
        Write-Host "   ⚠️  $var não definida" -ForegroundColor Yellow
        $missing += $var
    } else {
        $masked = $value.Substring(0, [Math]::Min(8, $value.Length)) + "..." 
        Write-Host "   ✅ $var = $masked" -ForegroundColor Green
    }
}

if ($missing.Count -gt 0) {
    Write-Host "`n   ⚠️  Variáveis faltando: $($missing -join ', ')" -ForegroundColor Yellow
    Write-Host "   Configure-as antes de usar o Jarvis CLI" -ForegroundColor Gray
}

# 3. Verificar Jarvis CLI
Write-Host "`n3. Verificando instalação do Jarvis CLI..." -ForegroundColor Yellow
$jarvis = Get-Command jarvis -ErrorAction SilentlyContinue
if ($jarvis) {
    Write-Host "   ✅ Jarvis CLI encontrado" -ForegroundColor Green
    Write-Host "   📍 Localização: $($jarvis.Source)" -ForegroundColor Gray
    Write-Host "`n   Testando versão..." -ForegroundColor Gray
    try {
        $version = & jarvis --version 2>&1
        Write-Host "   $version" -ForegroundColor Cyan
    } catch {
        Write-Host "   ⚠️  Erro ao obter versão" -ForegroundColor Yellow
    }
} else {
    Write-Host "   ⚠️  Jarvis CLI não encontrado no PATH" -ForegroundColor Yellow
    Write-Host "   Certifique-se de que está instalado e no PATH" -ForegroundColor Gray
}

# 4. Resumo
Write-Host "`n=== Resumo ===" -ForegroundColor Cyan
Write-Host "✅ Configuração básica verificada" -ForegroundColor Green

if ($missing.Count -eq 0 -and $jarvis) {
    Write-Host "`n🎉 Tudo pronto! Você pode testar com:" -ForegroundColor Green
    Write-Host "   jarvis chat" -ForegroundColor Cyan
} else {
    Write-Host "`n⚠️  Ações necessárias:" -ForegroundColor Yellow
    if ($missing.Count -gt 0) {
        Write-Host "   1. Configure as variáveis de ambiente faltantes" -ForegroundColor White
    }
    if (-not $jarvis) {
        Write-Host "   2. Instale o Jarvis CLI" -ForegroundColor White
    }
}

Write-Host ""
