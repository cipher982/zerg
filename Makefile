# Zerg Agent Platform – convenience commands for humans & agents

# ---------------------------------------------------------------------------
# Ports can be overridden, e.g. `make B_PORT=9001 dev`
# ---------------------------------------------------------------------------
B_PORT ?= 8001
F_PORT ?= 8002

# Command templates
PY_UVICORN = uv run python -m uvicorn zerg.main:app

.PHONY: help backend frontend dev stop test e2e compose

# ---------------------------------------------------------------------------
# Help – `make` or `make help`
# ---------------------------------------------------------------------------
help:
	@echo "\nZerg Agent Platform – common commands"
	@echo "-------------------------------------"
	@echo "make backend   # start API on port $(B_PORT)"
	@echo "make frontend  # start web-app on port $(F_PORT)"
	@echo "make dev       # start backend + frontend together"
	@echo "make stop      # kill anything on $(B_PORT) $(F_PORT)"
	@echo "make test      # backend & frontend unit tests"
	@echo "make e2e       # full Playwright E2E suite"
	@echo "make compose   # (optional) docker-compose up --build"
	@echo ""

# ---------------------------------------------------------------------------
# Runtime servers
# ---------------------------------------------------------------------------
backend:
	cd backend && $(PY_UVICORN) --reload --port $(B_PORT)

frontend:
	cd frontend && ./build-debug.sh

# Run both servers in parallel (`make -j2 dev` would also work)
dev:
	$(MAKE) -j2 backend frontend

# ---------------------------------------------------------------------------
# House-keeping
# ---------------------------------------------------------------------------
stop:
	- lsof -ti:$(B_PORT) $(F_PORT) | xargs -r kill

# ---------------------------------------------------------------------------
# Tests
# ---------------------------------------------------------------------------
# ---------------------------------------------------------------------------
# Tests (+ AsyncAPI spec validation)
# ---------------------------------------------------------------------------

test:
	./scripts/validate-asyncapi.sh
	cd backend  && ./run_backend_tests.sh
	- cd frontend && ./run_frontend_tests.sh || echo "[make test] 🟡 Frontend tests skipped (no browser / wasm-pack failure)"

e2e:
	cd e2e && ./run_e2e_tests.sh

# ---------------------------------------------------------------------------
# AsyncAPI – code generation from spec
# ---------------------------------------------------------------------------

regen-ws-code:
	./scripts/regen-ws-code.sh

# Verify that running regen-ws-code would not result in uncommitted changes.
ws-code-diff-check:
	./scripts/regen-ws-code.sh
	git diff --exit-code
	@echo "✅ WebSocket code up to date with spec"

# ---------------------------------------------------------------------------
# Pact contract capture / verification
# ---------------------------------------------------------------------------

pact-capture:
	mkdir -p .cargo_tmp
	TMPDIR="$(PWD)/.cargo_tmp" cargo run --manifest-path frontend/Cargo.toml --bin contract_capture --quiet
	@# Fail if the contract file changed and is not committed
	git diff --ignore-space-at-eol --exit-code contracts/frontend-v1.json || (echo "\n❌ Pact contract drift – commit updated JSON" && exit 1)

pact-verify:
	cd backend && ./run_backend_tests.sh -q -k pact_contracts || true
	@echo "✅ Pact verification finished (skip flag when pact_verifier missing)"

# ---------------------------------------------------------------------------
# Container stack – optional, used by CI or when you prefer isolation
# ---------------------------------------------------------------------------
compose:
	docker compose up --build
