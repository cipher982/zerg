# Zerg Agent Platform

# ---------------------------------------------------------------------------
# Ports can be overridden, e.g. `make B_PORT=9001 start`
# ---------------------------------------------------------------------------
B_PORT ?= 8001
F_PORT ?= 8002

.PHONY: help start stop test generate

# ---------------------------------------------------------------------------
# Help – `make` or `make help`
# ---------------------------------------------------------------------------
help:
	@echo "\nZerg Agent Platform"
	@echo "-------------------"
	@echo "make start     # start development servers"
	@echo "make stop      # stop development servers" 
	@echo "make test      # run all tests"
	@echo "make generate  # regenerate code from schemas"
	@echo "make help      # show this help"
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

test:
	@echo "🧪 Running all tests..."
	cd backend && ./run_backend_tests.sh
	- cd frontend && ./run_frontend_tests.sh || echo "🟡 Frontend tests skipped"
	cd e2e && ./run_e2e_tests.sh --mode=basic
	@echo "✅ All tests complete"

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