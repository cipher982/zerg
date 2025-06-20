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

# Avoid sandbox tmp permission issues in restrictive environments
export PLAYWRIGHT_CACHE_DIR="${SCRIPT_DIR}/.pw_cache"
mkdir -p "$PLAYWRIGHT_CACHE_DIR"
export TMPDIR="${SCRIPT_DIR}/.pw_tmp"
mkdir -p "$TMPDIR"
export PW_TMP_DIR="$TMPDIR"

# Reduce backend & frontend noise so CI logs stay readable.  The backend
# honours this flag via `zerg.main` (see *silence_info_logs()*), muting
# *INFO* level messages and above for most internal loggers.  Front-end
# build output remains unchanged; Playwright’s coloured reporter will still
# show failing test details.
export E2E_LOG_SUPPRESS=1

# Pass through all given args to the underlying npm command
npm test -- --reporter=dot "$@"
TEST_EXIT_CODE=$?

# Clean up worker-specific test databases
echo "[run_e2e_tests] Cleaning up test databases..." >&2
if [ -f "../backend/cleanup_test_dbs.sh" ]; then
  (cd ../backend && ./cleanup_test_dbs.sh)
fi

echo "[run_e2e_tests] ✔ complete" >&2
exit $TEST_EXIT_CODE
