#!/usr/bin/env bash
# ---------------------------------------------------------------------------
# End-to-End Test Runner
# ---------------------------------------------------------------------------
# 1. Make sure nothing is already using the Playwright ports (8001 backend,
#    8002 frontend) – stray dev servers are the usual culprit.
# 2. Delegate all server start-up to Playwright via playwright.config.js.
# 3. Forward any extra CLI flags to Playwright (`./run_e2e_tests.sh --grep @tag`).
# ---------------------------------------------------------------------------
set -euo pipefail

# Ensure we run from the e2e directory regardless of where the script was
# invoked from.
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR"

# Shut down any process that might occupy the ports Playwright needs.
for PORT in 8001 8002; do
  if lsof -i:"$PORT" >/dev/null 2>&1; then
    echo "[run_e2e_tests] Killing process on port $PORT …" >&2
    lsof -ti:"$PORT" | xargs -r kill -9 || true
  fi
done

echo "[run_e2e_tests] Running Playwright tests…" >&2

# Pass through all given args to the underlying npm command
npm test -- "$@"

echo "[run_e2e_tests] ✔ complete" >&2
