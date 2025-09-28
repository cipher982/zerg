#!/bin/bash

# UI Comparison Test Runner
# Executes E2E tests against both Rust and React UIs and displays results

set -e

echo "🚀 UI Comparison Test Suite"
echo "═══════════════════════════════════════════════════════════════════════════════"
echo ""

# Navigate to e2e directory
cd "$(dirname "$0")/e2e"

# Install dependencies if needed
if [ ! -d "node_modules" ]; then
    echo "📦 Installing E2E test dependencies..."
    npm install --quiet
fi

# Define test scenarios
declare -a TEST_FILES=(
    "agent_creation_full.spec.ts"
    "chat_functional.spec.ts"
    "dashboard.parity.spec.ts"
)

declare -a TEST_NAMES=(
    "Agent Creation"
    "Chat Functionality"
    "Dashboard Parity"
)

# Arrays to store results
declare -a RUST_RESULTS=()
declare -a REACT_RESULTS=()

echo "Running tests against both UIs..."
echo ""

# Test each scenario
for i in "${!TEST_FILES[@]}"; do
    FILE="${TEST_FILES[$i]}"
    NAME="${TEST_NAMES[$i]}"

    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo "Testing: $NAME"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

    # Test with Rust UI
    echo -n "  🦀 Rust UI:   "
    if PLAYWRIGHT_USE_RUST_UI=1 npx playwright test "tests/$FILE" --reporter=dot --quiet 2>/dev/null; then
        echo "✅ PASS"
        RUST_RESULTS+=("✅")
    else
        echo "❌ FAIL"
        RUST_RESULTS+=("❌")
    fi

    # Test with React UI
    echo -n "  ⚛️  React UI:  "
    if PLAYWRIGHT_USE_RUST_UI=0 npx playwright test "tests/$FILE" --reporter=dot --quiet 2>/dev/null; then
        echo "✅ PASS"
        REACT_RESULTS+=("✅")
    else
        echo "❌ FAIL"
        REACT_RESULTS+=("❌")
    fi

    echo ""
done

# Display summary table
echo "═══════════════════════════════════════════════════════════════════════════════"
echo "                              📊 RESULTS SUMMARY"
echo "═══════════════════════════════════════════════════════════════════════════════"
echo ""
echo "┌────────────────────────┬──────────────┬──────────────┐"
echo "│ Test Scenario          │ Rust/WASM UI │ React UI     │"
echo "├────────────────────────┼──────────────┼──────────────┤"

for i in "${!TEST_NAMES[@]}"; do
    printf "│ %-22s │ %-12s │ %-12s │\n" "${TEST_NAMES[$i]}" "${RUST_RESULTS[$i]} PASS" "${REACT_RESULTS[$i]} PASS"
done

echo "└────────────────────────┴──────────────┴──────────────┘"

# Calculate parity score
MATCHING=0
TOTAL=${#TEST_NAMES[@]}

for i in "${!TEST_NAMES[@]}"; do
    if [ "${RUST_RESULTS[$i]}" = "${REACT_RESULTS[$i]}" ]; then
        ((MATCHING++))
    fi
done

PARITY=$((MATCHING * 100 / TOTAL))

echo ""
echo "🎯 UI Parity Score: $PARITY% ($MATCHING/$TOTAL matching results)"
echo ""

# Provide recommendations
if [ $PARITY -eq 100 ]; then
    echo "✨ Perfect parity! Both UIs are functioning identically."
elif [ $PARITY -ge 75 ]; then
    echo "👍 Good parity. Minor differences exist but core functionality matches."
elif [ $PARITY -ge 50 ]; then
    echo "⚠️  Moderate parity. Several differences need attention."
else
    echo "❌ Low parity. Significant work needed to align the UIs."
fi

echo ""
echo "═══════════════════════════════════════════════════════════════════════════════"
echo ""

# Exit with error if any tests failed
for result in "${RUST_RESULTS[@]}" "${REACT_RESULTS[@]}"; do
    if [ "$result" = "❌" ]; then
        exit 1
    fi
done

exit 0