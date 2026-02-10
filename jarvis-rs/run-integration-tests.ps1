#!/usr/bin/env pwsh
# Integration Tests Runner for Jarvis CLI
# This script starts Docker containers and runs integration tests

param(
    [switch]$SkipBuild,
    [switch]$KeepContainers,
    [string]$Filter = ""
)

$ErrorActionPreference = "Stop"

Write-Host "🚀 Jarvis CLI - Integration Tests Runner" -ForegroundColor Cyan
Write-Host "=========================================" -ForegroundColor Cyan
Write-Host ""

# Check if Docker is running
Write-Host "🔍 Checking Docker..." -ForegroundColor Yellow
try {
    docker info | Out-Null
    Write-Host "✅ Docker is running" -ForegroundColor Green
} catch {
    Write-Host "❌ Docker is not running. Please start Docker Desktop." -ForegroundColor Red
    exit 1
}

# Start test containers
Write-Host ""
Write-Host "🐳 Starting test containers..." -ForegroundColor Yellow
docker-compose -f docker-compose.test.yml up -d

# Wait for services to be healthy
Write-Host ""
Write-Host "⏳ Waiting for services to be ready..." -ForegroundColor Yellow

$maxAttempts = 30
$attempt = 0

while ($attempt -lt $maxAttempts) {
    $attempt++

    $sqlserverHealthy = docker inspect --format='{{.State.Health.Status}}' jarvis-test-sqlserver 2>$null
    $redisHealthy = docker inspect --format='{{.State.Health.Status}}' jarvis-test-redis 2>$null
    $qdrantHealthy = docker inspect --format='{{.State.Health.Status}}' jarvis-test-qdrant 2>$null

    if ($sqlserverHealthy -eq "healthy" -and $redisHealthy -eq "healthy" -and $qdrantHealthy -eq "healthy") {
        Write-Host "✅ All services are healthy!" -ForegroundColor Green
        break
    }

    Write-Host "   Attempt $attempt/$maxAttempts - SQL Server: $sqlserverHealthy, Redis: $redisHealthy, Qdrant: $qdrantHealthy" -ForegroundColor Gray
    Start-Sleep -Seconds 2
}

if ($attempt -ge $maxAttempts) {
    Write-Host "❌ Services failed to become healthy in time" -ForegroundColor Red
    docker-compose -f docker-compose.test.yml logs
    docker-compose -f docker-compose.test.yml down
    exit 1
}

# Set environment variables for tests
$env:JARVIS_TEST_SQLSERVER_CONN = "Server=localhost,1433;Database=jarvis_test;User Id=sa;Password=YourPassword123!;TrustServerCertificate=True"
$env:JARVIS_TEST_REDIS_URL = "redis://localhost:6379"
$env:JARVIS_TEST_QDRANT_URL = "http://localhost:6333"

# Run integration tests
Write-Host ""
Write-Host "🧪 Running integration tests..." -ForegroundColor Yellow
Write-Host ""

$testCommand = "cargo test --package jarvis-core --lib -- --ignored --nocapture"
if ($Filter -ne "") {
    $testCommand += " $Filter"
}

Write-Host "Command: $testCommand" -ForegroundColor Gray
Write-Host ""

try {
    Invoke-Expression $testCommand
    $testResult = $LASTEXITCODE
} catch {
    $testResult = 1
}

# Cleanup
if (-not $KeepContainers) {
    Write-Host ""
    Write-Host "🧹 Cleaning up containers..." -ForegroundColor Yellow
    docker-compose -f docker-compose.test.yml down
    Write-Host "✅ Cleanup complete" -ForegroundColor Green
} else {
    Write-Host ""
    Write-Host "⚠️  Containers are still running (--KeepContainers flag)" -ForegroundColor Yellow
    Write-Host "   To stop them manually: docker-compose -f docker-compose.test.yml down" -ForegroundColor Gray
}

# Exit with test result
Write-Host ""
if ($testResult -eq 0) {
    Write-Host "✅ All integration tests passed!" -ForegroundColor Green
} else {
    Write-Host "❌ Some integration tests failed" -ForegroundColor Red
}

exit $testResult
