#!/bin/bash
# Integration Tests Runner for Jarvis CLI
# This script starts Docker containers and runs integration tests

set -e

SKIP_BUILD=false
KEEP_CONTAINERS=false
FILTER=""

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --skip-build)
            SKIP_BUILD=true
            shift
            ;;
        --keep-containers)
            KEEP_CONTAINERS=true
            shift
            ;;
        --filter)
            FILTER="$2"
            shift 2
            ;;
        *)
            echo "Unknown option: $1"
            echo "Usage: $0 [--skip-build] [--keep-containers] [--filter PATTERN]"
            exit 1
            ;;
    esac
done

echo "🚀 Jarvis CLI - Integration Tests Runner"
echo "========================================="
echo ""

# Check if Docker is running
echo "🔍 Checking Docker..."
if ! docker info > /dev/null 2>&1; then
    echo "❌ Docker is not running. Please start Docker."
    exit 1
fi
echo "✅ Docker is running"

# Start test containers
echo ""
echo "🐳 Starting test containers..."
docker-compose -f docker-compose.test.yml up -d

# Wait for services to be healthy
echo ""
echo "⏳ Waiting for services to be ready..."

MAX_ATTEMPTS=30
ATTEMPT=0

while [ $ATTEMPT -lt $MAX_ATTEMPTS ]; do
    ATTEMPT=$((ATTEMPT + 1))

    SQLSERVER_HEALTHY=$(docker inspect --format='{{.State.Health.Status}}' jarvis-test-sqlserver 2>/dev/null || echo "unknown")
    REDIS_HEALTHY=$(docker inspect --format='{{.State.Health.Status}}' jarvis-test-redis 2>/dev/null || echo "unknown")
    QDRANT_HEALTHY=$(docker inspect --format='{{.State.Health.Status}}' jarvis-test-qdrant 2>/dev/null || echo "unknown")

    if [ "$SQLSERVER_HEALTHY" = "healthy" ] && [ "$REDIS_HEALTHY" = "healthy" ] && [ "$QDRANT_HEALTHY" = "healthy" ]; then
        echo "✅ All services are healthy!"
        break
    fi

    echo "   Attempt $ATTEMPT/$MAX_ATTEMPTS - SQL Server: $SQLSERVER_HEALTHY, Redis: $REDIS_HEALTHY, Qdrant: $QDRANT_HEALTHY"
    sleep 2
done

if [ $ATTEMPT -ge $MAX_ATTEMPTS ]; then
    echo "❌ Services failed to become healthy in time"
    docker-compose -f docker-compose.test.yml logs
    docker-compose -f docker-compose.test.yml down
    exit 1
fi

# Set environment variables for tests
export JARVIS_TEST_SQLSERVER_CONN="Server=localhost,1433;Database=jarvis_test;User Id=sa;Password=YourPassword123!;TrustServerCertificate=True"
export JARVIS_TEST_REDIS_URL="redis://localhost:6379"
export JARVIS_TEST_QDRANT_URL="http://localhost:6333"

# Run integration tests
echo ""
echo "🧪 Running integration tests..."
echo ""

TEST_COMMAND="cargo test --package jarvis-core --lib -- --ignored --nocapture"
if [ -n "$FILTER" ]; then
    TEST_COMMAND="$TEST_COMMAND $FILTER"
fi

echo "Command: $TEST_COMMAND"
echo ""

set +e
eval "$TEST_COMMAND"
TEST_RESULT=$?
set -e

# Cleanup
if [ "$KEEP_CONTAINERS" = false ]; then
    echo ""
    echo "🧹 Cleaning up containers..."
    docker-compose -f docker-compose.test.yml down
    echo "✅ Cleanup complete"
else
    echo ""
    echo "⚠️  Containers are still running (--keep-containers flag)"
    echo "   To stop them manually: docker-compose -f docker-compose.test.yml down"
fi

# Exit with test result
echo ""
if [ $TEST_RESULT -eq 0 ]; then
    echo "✅ All integration tests passed!"
else
    echo "❌ Some integration tests failed"
fi

exit $TEST_RESULT
