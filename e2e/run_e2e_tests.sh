#!/usr/bin/env bash
# ---------------------------------------------------------------------------
# E2E Test Runner – Execute end-to-end tests with Playwright
# ---------------------------------------------------------------------------
# This script runs the full E2E test suite using Playwright. It will:
# 1. Install dependencies if needed
# 2. Start backend and frontend servers automatically (via playwright config)
# 3. Run all E2E tests in headless mode
# ---------------------------------------------------------------------------
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$ROOT_DIR"

echo "[run_e2e_tests] Starting E2E test suite..." >&2

# Check if node_modules exists, if not install dependencies
if [[ ! -d "node_modules" ]]; then
    echo "[run_e2e_tests] Installing dependencies..." >&2
    npm install
fi

# Run Playwright tests
echo "[run_e2e_tests] Running Playwright tests..." >&2
npx playwright test

echo "[run_e2e_tests] ✔ E2E tests completed successfully" >&2
