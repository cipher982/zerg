# Zerg Agent Platform

# ---------------------------------------------------------------------------
# Load environment variables from .env (ports are now configured there)
# ---------------------------------------------------------------------------
include .env
export $(shell sed 's/=.*//' .env)

# Fallback defaults if .env is missing values
B_PORT ?= $(BACKEND_PORT)
F_PORT ?= $(FRONTEND_PORT)
B_PORT ?= 8001
F_PORT ?= 8002

.PHONY: help start stop test test-backend test-frontend test-e2e test-auto test-ci generate validate-contracts validate-deploy

# ---------------------------------------------------------------------------
# Help – `make` or `make help`
# ---------------------------------------------------------------------------
help:
	@echo "\nZerg Agent Platform"
	@echo "==================="
	@echo ""
	@echo "Development:"
	@echo "  make start         Start backend (port $(B_PORT)) and frontend (port $(F_PORT)) servers"
	@echo "  make stop          Stop all development servers"
	@echo "  make generate      Regenerate code from AsyncAPI schemas"
	@echo "  make validate-contracts  Run contract validation checks (catches API mismatches)"
	@echo ""
	@echo "Testing:"
	@echo "  make test          Run ALL tests (backend + frontend + e2e)"
	@echo "  make test-backend  Run backend unit tests only (~10 sec)"
	@echo "  make test-frontend Run frontend WASM tests only (~30 sec)"
	@echo "  make test-e2e      Run e2e integration tests only (~2 min)"
	@echo "  make test-auto     🤖 Automated UI parity testing (zero human interaction)"
	@echo "  make test-ci       🚀 CI-ready test suite (unit tests + builds + contracts)"
	@echo ""
	@echo "Deployment:"
	@echo "  make validate-deploy    Validate environment for deployment (required vars, DB connectivity)"
	@echo ""

# ---------------------------------------------------------------------------
# Development workflow
# ---------------------------------------------------------------------------
start:
	@echo "🚀 Starting development servers on ports $(B_PORT) and $(F_PORT)..."
	./scripts/fast-contract-check.sh
	$(MAKE) -j2 _backend _frontend

_backend:
	cd backend && uv run python -m uvicorn zerg.main:app --reload --port $(B_PORT)

_frontend:
	cd frontend && ./build-debug.sh

stop:
	- lsof -ti:$(B_PORT) | xargs kill 2>/dev/null || true
	- lsof -ti:$(F_PORT) | xargs kill 2>/dev/null || true

# ---------------------------------------------------------------------------
# Testing targets
# ---------------------------------------------------------------------------

test:
	@echo "🧪 Running ALL tests..."
	cd backend && ./run_backend_tests.sh
	cd frontend && ./run_frontend_tests.sh
	cd e2e && ./run_e2e_tests.sh --mode=basic
	@echo "✅ All tests complete"

test-backend:
	@echo "🧪 Running backend unit tests..."
	cd backend && ./run_backend_tests.sh

test-frontend:
	@echo "🧪 Running frontend WASM tests..."
	cd frontend && ./run_frontend_tests.sh

test-e2e:
	@echo "🧪 Running E2E integration tests..."
	cd e2e && ./run_e2e_tests.sh --mode=basic

test-auto:
	@echo "🤖 Running automated UI parity tests (zero human interaction)..."
	./run-automated-tests.sh

test-ci:
	@echo "🚀 Running CI-ready test suite (unit tests + builds + contracts)..."
	./run-ci-tests.sh

# ---------------------------------------------------------------------------
# Code generation (run when schemas change)
# ---------------------------------------------------------------------------
generate:
	@echo "🔄 Regenerating code from schemas..."
	@echo "📡 WebSocket types..."
	python3 scripts/generate-ws-types-modern.py ws-protocol-asyncapi.yml
	@echo "🛠  Tool types..."
	python3 scripts/generate_tool_types.py asyncapi/tools.yml
	@echo "🔍 Validating..."
	python3 scripts/validate_tool_contracts.py
	./scripts/validate-asyncapi.sh
	@echo "✅ Code generation complete"

# ---------------------------------------------------------------------------
# Contract validation (catches API mismatches at build time)
# ---------------------------------------------------------------------------
validate-contracts:
	@echo "🔍 Running API contract validation..."
	./scripts/fast-contract-check.sh

# ---------------------------------------------------------------------------
# Deployment validation (checks environment and connectivity)
# ---------------------------------------------------------------------------
validate-deploy:
	@echo "🔍 Validating deployment configuration..."
	cd backend && uv run python ../scripts/validate-deployment.py