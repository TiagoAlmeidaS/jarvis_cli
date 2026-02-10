.PHONY: help test test-unit test-integration test-all docker-up docker-down docker-logs clean

# Default target
help:
	@echo "Jarvis CLI - Test Commands"
	@echo "==========================="
	@echo ""
	@echo "Available targets:"
	@echo "  make test            - Run all unit tests (fast)"
	@echo "  make test-unit       - Run unit tests only"
	@echo "  make test-integration - Run integration tests (requires Docker)"
	@echo "  make test-all        - Run both unit and integration tests"
	@echo "  make docker-up       - Start Docker test services"
	@echo "  make docker-down     - Stop Docker test services"
	@echo "  make docker-logs     - Show Docker logs"
	@echo "  make clean           - Clean build artifacts and Docker containers"
	@echo ""

# Run unit tests only
test test-unit:
	@echo "🧪 Running unit tests..."
	cargo test --package jarvis-core --lib

# Start Docker services
docker-up:
	@echo "🐳 Starting Docker test services..."
	docker-compose -f docker-compose.test.yml up -d
	@echo "⏳ Waiting for services to be healthy..."
	@sleep 10

# Stop Docker services
docker-down:
	@echo "🛑 Stopping Docker test services..."
	docker-compose -f docker-compose.test.yml down

# Show Docker logs
docker-logs:
	docker-compose -f docker-compose.test.yml logs -f

# Run integration tests (requires Docker to be running)
test-integration: docker-up
	@echo "🧪 Running integration tests..."
	@export $$(cat .env.test | xargs) && \
		cargo test --package jarvis-core --lib -- --ignored --nocapture || \
		(make docker-down && exit 1)
	@make docker-down

# Run all tests
test-all: test-unit test-integration

# Clean everything
clean: docker-down
	@echo "🧹 Cleaning build artifacts..."
	cargo clean
	@echo "✅ Clean complete"

# Quick integration test (keeps containers running)
test-integration-quick:
	@echo "🧪 Running integration tests (quick mode)..."
	@export $$(cat .env.test | xargs) && \
		cargo test --package jarvis-core --lib -- --ignored --nocapture
