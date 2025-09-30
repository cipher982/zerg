#!/bin/bash

# Automated Test Suite - Zero Human Interaction Required
# Starts backend, runs core UI parity tests, displays results, cleans up

set -e

echo "🤖 Automated Test Suite Starting..."
echo "═══════════════════════════════════════════════════════════════════════════════"
echo ""

# Ensure we're in project root
cd "$(dirname "$0")"

# Kill any existing processes on our ports first
echo "🧹 Cleaning up existing processes..."
pkill -f "port=47300" || true
pkill -f "port=47200" || true
sleep 2

echo "🚀 Starting backend and frontend services..."

# Start backend in the background
echo "  📡 Starting backend on port 47300..."
cd backend && uv run python -m uvicorn zerg.main:app --host=127.0.0.1 --port=47300 --log-level=warning > ../test-backend.log 2>&1 &
BACKEND_PID=$!
cd ..

# Give backend time to start
echo "  ⏳ Waiting for backend to initialize..."
sleep 5

# Verify backend is responding
if ! curl -s http://localhost:47300/health > /dev/null; then
    echo "❌ Backend failed to start!"
    kill $BACKEND_PID || true
    exit 1
fi

echo "  ✅ Backend ready"

# Function to cleanup on exit
cleanup() {
    echo "🧹 Cleaning up..."
    kill $BACKEND_PID || true
    pkill -f "port=47200" || true
    rm -f test-backend.log
}
trap cleanup EXIT

# Run quick smoke tests for both UIs
echo ""
echo "🎯 Running Core UI Parity Tests..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

# Change to e2e directory
if ! cd e2e; then
    echo "❌ E2E directory not found!"
    exit 1
fi

# Test key scenarios only
declare -a QUICK_TESTS=(
    "dashboard.parity.spec.ts"
    "chat_functional.spec.ts"
)

declare -a TEST_NAMES=(
    "Dashboard Parity"
    "Chat Functionality"
)

# Results storage
declare -a RUST_RESULTS=()
declare -a REACT_RESULTS=()

# Run tests for each UI
for i in "${!QUICK_TESTS[@]}"; do
    FILE="${QUICK_TESTS[$i]}"
    NAME="${TEST_NAMES[$i]}"

    echo ""
    echo "Testing: $NAME"
    echo "────────────────────────────────────────────────────────────────────────────"

    # Test Rust UI
    echo -n "  🦀 Rust UI:   "
    if PLAYWRIGHT_USE_RUST_UI=1 timeout 60 npx playwright test "tests/$FILE" --reporter=dot --quiet > /dev/null 2>&1; then
        echo "✅ PASS"
        RUST_RESULTS+=("✅")
    else
        echo "❌ FAIL"
        RUST_RESULTS+=("❌")
    fi

    # Test React UI
    echo -n "  ⚛️  React UI:  "
    if PLAYWRIGHT_USE_RUST_UI=0 timeout 60 npx playwright test "tests/$FILE" --reporter=dot --quiet > /dev/null 2>&1; then
        echo "✅ PASS"
        REACT_RESULTS+=("✅")
    else
        echo "❌ FAIL"
        REACT_RESULTS+=("❌")
    fi
done

cd ..

# Results summary
echo ""
echo "═══════════════════════════════════════════════════════════════════════════════"
echo "                              📊 RESULTS SUMMARY"
echo "═══════════════════════════════════════════════════════════════════════════════"
echo ""
echo "┌────────────────────────┬──────────────┬──────────────┐"
echo "│ Test Scenario          │ Rust/WASM UI │ React UI     │"
echo "├────────────────────────┼──────────────┼──────────────┤"

for i in "${!TEST_NAMES[@]}"; do
    RUST_STATUS="${RUST_RESULTS[$i]}"
    REACT_STATUS="${REACT_RESULTS[$i]}"
    printf "│ %-22s │ %s %-8s │ %s %-8s │\n" "${TEST_NAMES[$i]}" "$RUST_STATUS" "PASS" "$REACT_STATUS" "PASS"
done

echo "└────────────────────────┴──────────────┴──────────────┘"

# Calculate parity
MATCHING=0
TOTAL=${#TEST_NAMES[@]}

for i in "${!TEST_NAMES[@]}"; do
    if [ "${RUST_RESULTS[$i]}" = "${REACT_RESULTS[$i]}" ]; then
        ((MATCHING++))
    fi
done

PARITY=$((MATCHING * 100 / TOTAL))

echo ""
echo "🎯 UI Parity Score: $PARITY% ($MATCHING/$TOTAL scenarios matching)"

if [ $PARITY -eq 100 ]; then
    echo "✨ Perfect parity! Both UIs behaving identically."
elif [ $PARITY -ge 75 ]; then
    echo "👍 Good parity. Minor differences need attention."
else
    echo "⚠️  Parity issues detected. UI synchronization needed."
fi

echo ""
echo "🤖 Automated testing complete! No human interaction required."
echo "═══════════════════════════════════════════════════════════════════════════════"

# Exit with error if any tests failed
for result in "${RUST_RESULTS[@]}" "${REACT_RESULTS[@]}"; do
    if [ "$result" = "❌" ]; then
        exit 1
    fi
done

exit 0