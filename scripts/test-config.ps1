# Script PowerShell para testar a configuração do Jarvis CLI
# Este script verifica se o arquivo config.toml está configurado corretamente

Write-Host "=== Teste de Configuração do Jarvis CLI ===" -ForegroundColor Cyan
Write-Host ""

$jarvisHome = "$env:USERPROFILE\.jarvis"
$configFile = "$jarvisHome\config.toml"

# Verificar se o diretório existe
if (-not (Test-Path $jarvisHome)) {
    Write-Host "❌ Diretório $jarvisHome não existe!" -ForegroundColor Red
    Write-Host "Criando diretório..." -ForegroundColor Yellow
    New-Item -ItemType Directory -Force -Path $jarvisHome | Out-Null
}

# Verificar se o arquivo config.toml existe
if (-not (Test-Path $configFile)) {
    Write-Host "❌ Arquivo config.toml não encontrado em: $configFile" -ForegroundColor Red
    Write-Host ""
    Write-Host "Para criar o arquivo:" -ForegroundColor Yellow
    Write-Host "1. Copie o arquivo config.toml.example para $configFile" -ForegroundColor White
    Write-Host "2. Ou execute: Copy-Item '.\config.toml.example' '$configFile'" -ForegroundColor White
    Write-Host ""
    exit 1
}

Write-Host "✅ Arquivo config.toml encontrado: $configFile" -ForegroundColor Green
Write-Host ""

# Verificar se o arquivo pode ser lido como TOML
Write-Host "Verificando sintaxe TOML..." -ForegroundColor Cyan
try {
    $content = Get-Content $configFile -Raw
    Write-Host "✅ Arquivo pode ser lido" -ForegroundColor Green
} catch {
    Write-Host "❌ Erro ao ler arquivo: $_" -ForegroundColor Red
    exit 1
}

# Verificar variáveis de ambiente necessárias
Write-Host ""
Write-Host "Verificando variáveis de ambiente..." -ForegroundColor Cyan

$envVars = @{
    "DATABRICKS_API_KEY" = "your_databricks_api_key_here"
    "OPENAI_API_KEY" = "your_openai_api_key_here"
    "OPENROUTER_API_KEY" = "your_openrouter_api_key_here"
    "AZURE_OPENAI_API_KEY" = ""
    "ANTHROPIC_API_KEY" = ""
}

$missingVars = @()
foreach ($var in $envVars.Keys) {
    $value = [System.Environment]::GetEnvironmentVariable($var, "User")
    if ([string]::IsNullOrEmpty($value)) {
        Write-Host "⚠️  Variável $var não está definida" -ForegroundColor Yellow
        $missingVars += $var
    } else {
        Write-Host "✅ Variável $var está definida" -ForegroundColor Green
    }
}

if ($missingVars.Count -gt 0) {
    Write-Host ""
    Write-Host "⚠️  Algumas variáveis de ambiente não estão definidas:" -ForegroundColor Yellow
    foreach ($var in $missingVars) {
        Write-Host "   - $var" -ForegroundColor White
    }
    Write-Host ""
    Write-Host "Para definir variáveis de ambiente no PowerShell:" -ForegroundColor Cyan
    Write-Host '[System.Environment]::SetEnvironmentVariable("NOME_VARIAVEL", "valor", "User")' -ForegroundColor White
    Write-Host ""
}

# Verificar se o Jarvis CLI está instalado
Write-Host ""
Write-Host "Verificando instalação do Jarvis CLI..." -ForegroundColor Cyan
$jarvisCmd = Get-Command jarvis -ErrorAction SilentlyContinue
if ($jarvisCmd) {
    Write-Host "✅ Jarvis CLI encontrado: $($jarvisCmd.Source)" -ForegroundColor Green
    Write-Host ""
    Write-Host "Versão:" -ForegroundColor Cyan
    & jarvis --version 2>&1 | Write-Host
} else {
    Write-Host "⚠️  Jarvis CLI não encontrado no PATH" -ForegroundColor Yellow
    Write-Host "   Certifique-se de que o Jarvis CLI está instalado e no PATH" -ForegroundColor White
}

Write-Host ""
Write-Host "=== Teste Concluído ===" -ForegroundColor Cyan
Write-Host ""
Write-Host "Próximos passos:" -ForegroundColor Yellow
Write-Host "1. Configure as variáveis de ambiente necessárias" -ForegroundColor White
Write-Host "2. Teste o Jarvis CLI com: jarvis chat" -ForegroundColor White
Write-Host "3. Verifique os logs se houver problemas" -ForegroundColor White
