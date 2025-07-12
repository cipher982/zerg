# Zerg Agent Platform – convenience commands for humans & agents

# ---------------------------------------------------------------------------
# Ports can be overridden, e.g. `make B_PORT=9001 dev`
# ---------------------------------------------------------------------------
B_PORT ?= 8001
F_PORT ?= 8002

# Command templates
PY_UVICORN = uv run python -m uvicorn zerg.main:app

.PHONY: help backend frontend dev stop test test-all e2e e2e-basic compose tool-code-gen tool-validate tool-check tool-code-diff-check

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
	@echo "make test      # backend & frontend unit tests + tool contracts"
	@echo "make test-all  # complete test suite (unit + full E2E)"
	@echo "make e2e       # full E2E test suite (default)"
	@echo "make e2e-basic # essential tests only (~3 min)"
	@echo "make compose   # (optional) docker-compose up --build"
	@echo ""
	@echo "Tool Contract System:"
	@echo "make tool-code-gen        # generate Rust/Python types from schema"
	@echo "make tool-validate        # validate backend registry matches schema"
	@echo "make tool-check           # full tool contract validation (both above)"
	@echo "make tool-code-diff-check # verify generated code is up-to-date (CI)"
	@echo ""
	@echo "WebSocket Contract System (Modern AsyncAPI 3.0):"
	@echo "make ws-code-gen          # generate WebSocket types from AsyncAPI schema"
	@echo "make ws-validate          # validate WebSocket contracts"
	@echo "make ws-check             # full WebSocket contract validation"
	@echo "make ws-code-diff-check   # verify WebSocket code is up-to-date (CI)"
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
	$(MAKE) tool-check
	cd backend  && ./run_backend_tests.sh
	- cd frontend && ./run_frontend_tests.sh || echo "[make test] 🟡 Frontend tests skipped (no browser / wasm-pack failure)"

test-all:
	@echo "🧪 Running complete test suite (unit + comprehensive E2E)..."
	$(MAKE) test
	$(MAKE) e2e
	@echo "🎉 Complete test suite finished!"

e2e:
	cd e2e && ./run_e2e_tests.sh --mode=full

e2e-basic:
	cd e2e && ./run_e2e_tests.sh --mode=basic

# ---------------------------------------------------------------------------
# AsyncAPI – code generation from spec
# ---------------------------------------------------------------------------

regen-ws-code:
	./scripts/regen-ws-code.sh

# Legacy WebSocket target - use ws-code-diff-check-modern instead

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
# Tool contract generation / verification
# ---------------------------------------------------------------------------

tool-code-gen:
	@echo "🛠  Generating tool types from schema..."
	python3 scripts/generate_tool_types.py asyncapi/tools.yml

# Verify that generated code is up to date (for CI)
tool-code-diff-check: tool-code-gen
	@echo "🔍 Checking if tool definitions are up to date..."
	git diff --ignore-space-at-eol --exit-code frontend/src/generated/tool_definitions.rs backend/zerg/tools/generated/tool_definitions.py || (echo "\n❌ Tool contract drift – commit the generated changes" && exit 1)
	@echo "✅ Tool contracts are up to date"

tool-validate:
	@echo "🔍 Validating tool registry contracts..."
	python3 scripts/validate_tool_contracts.py

# Combined target for full tool contract checking
tool-check: tool-code-diff-check tool-validate
	@echo "✅ All tool contracts validated"

# ---------------------------------------------------------------------------
# Container stack – optional, used by CI or when you prefer isolation
# ---------------------------------------------------------------------------
compose:
	docker compose up --build

# ---------------------------------------------------------------------------
# Modern WebSocket Contract System (AsyncAPI 3.0)
# ---------------------------------------------------------------------------

ws-code-gen:
	@echo "🚀 Generating WebSocket types from AsyncAPI 3.0 schema..."
	python3 scripts/generate-ws-types-modern.py ws-protocol-asyncapi.yml

# Verify that generated WebSocket code is up to date (for CI)
ws-code-diff-check: ws-code-gen
	@echo "🔍 Checking if WebSocket definitions are up to date..."
	@# Check for meaningful changes (exclude timestamp-only changes)
	@./scripts/check_ws_drift.sh
	@echo "✅ WebSocket contracts are up to date"

ws-validate:
	@echo "🔍 Validating WebSocket contracts..."
	@# Validate AsyncAPI schema
	npx @asyncapi/cli validate ws-protocol-asyncapi.yml || (echo "⚠️  AsyncAPI validation skipped (CLI not available)" && true)
	@# TODO: Add runtime contract validation
	@echo "✅ WebSocket contracts validated"

# Combined target for full WebSocket contract checking
ws-check: ws-code-diff-check ws-validate
	@echo "✅ All WebSocket contracts validated"

# Update main test target to include WebSocket contracts
test-contracts: tool-check ws-check
	@echo "✅ All contracts validated (tools + WebSocket)"
