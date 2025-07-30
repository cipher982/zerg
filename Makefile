# Zerg Agent Platform

# ---------------------------------------------------------------------------
# Ports can be overridden, e.g. `make B_PORT=9001 start`
# ---------------------------------------------------------------------------
B_PORT ?= 8001
F_PORT ?= 8002

.PHONY: help start stop test test-backend test-frontend test-e2e generate

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
	@echo ""
	@echo "Testing:"
	@echo "  make test          Run ALL tests (backend + frontend + e2e)"
	@echo "  make test-backend  Run backend unit tests only (~10 sec)"
	@echo "  make test-frontend Run frontend WASM tests only (~30 sec)"
	@echo "  make test-e2e      Run e2e integration tests only (~2 min)"
	@echo ""
	@echo "Details:"
	@echo "  Backend tests:     $(shell find backend/tests -name 'test_*.py' | wc -l | tr -d ' ') Python unit tests with pytest"
	@echo "  Frontend tests:    Rust WASM tests with wasm-bindgen-test"
	@echo "  E2E tests:         Playwright browser tests (Chrome/Firefox required)"
	@echo ""
	@echo "Common workflows:"
	@echo "  make test-backend  # Quick feedback during backend development"
	@echo "  make test          # Full confidence before committing"
	@echo ""

# ---------------------------------------------------------------------------
# Development workflow
# ---------------------------------------------------------------------------
start:
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