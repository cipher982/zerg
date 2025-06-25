#!/usr/bin/env bash
# ---------------------------------------------------------------------------
# UNIFIED E2E TEST RUNNER
# ---------------------------------------------------------------------------
# Smart, consolidated E2E testing system that replaces multiple test runners
# with a single, mode-aware test execution system.
#
# Usage:
#   ./run_unified_e2e.sh                    # Full comprehensive suite
#   ./run_unified_e2e.sh --mode=quick       # Quick validation tests
#   ./run_unified_e2e.sh --mode=debug       # Debug and diagnostic tests
#   ./run_unified_e2e.sh --mode=core        # Core functionality only
#   ./run_unified_e2e.sh --mode=advanced    # Advanced tests (perf, a11y, etc.)
# ---------------------------------------------------------------------------

set -euo pipefail

# Default configuration
MODE="full"
VERBOSE=false
PARALLEL=true
REPORTS_DIR="test-reports"
TIMESTAMP=$(date +"%Y%m%d_%H%M%S")

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Function to print colored output
print_color() {
    local color=$1
    local message=$2
    echo -e "${color}${message}${NC}"
}

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --mode=*)
            MODE="${1#*=}"
            shift
            ;;
        --verbose|-v)
            VERBOSE=true
            shift
            ;;
        --no-parallel)
            PARALLEL=false
            shift
            ;;
        --help|-h)
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --mode=MODE           Test mode: full, basic"
            echo "  --verbose, -v         Verbose output"
            echo "  --no-parallel         Disable parallel execution"
            echo "  --help, -h           Show this help"
            echo ""
            echo "Modes:"
            echo "  full                Full comprehensive test suite (default)"
            echo "  basic               Essential tests only (~3 min)"
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
done

# Ensure we run from the e2e directory
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR"

print_color $BLUE "🚀 Unified E2E Test Runner"
print_color $BLUE "=========================="
print_color $CYAN "Mode: $MODE"
print_color $CYAN "Timestamp: $TIMESTAMP"
echo ""

# Create reports directory
mkdir -p "$REPORTS_DIR"

# Test categories organized by mode
declare -A TEST_CATEGORIES

# Basic mode - essential tests only
TEST_CATEGORIES[basic]="agent_creation_full.spec.ts comprehensive_debug.spec.ts canvas_complete_workflow.spec.ts"

# Full mode - everything
TEST_CATEGORIES[full]="agent_creation_full.spec.ts comprehensive_debug.spec.ts canvas_complete_workflow.spec.ts workflow_execution_http.spec.ts tool_palette_node_connections.spec.ts realtime_websocket_monitoring.spec.ts error_handling_edge_cases.spec.ts data_persistence_recovery.spec.ts performance_load_testing.spec.ts accessibility_ui_ux.spec.ts multi_user_concurrency.spec.ts"

# Validate mode
if [[ ! -v TEST_CATEGORIES[$MODE] ]]; then
    print_color $RED "❌ Invalid mode: $MODE"
    print_color $YELLOW "Valid modes: ${!TEST_CATEGORIES[*]}"
    exit 1
fi

# Pre-flight checks
print_color $YELLOW "🔧 Pre-flight checks..."

# Check if Playwright is installed
if ! command -v npx &> /dev/null; then
    print_color $RED "❌ npx not found. Please install Node.js and npm."
    exit 1
fi

# Check if Playwright config exists
if [ ! -f "playwright.config.ts" ]; then
    print_color $RED "❌ Playwright config not found at playwright.config.ts"
    exit 1
fi

# Kill any processes on test ports
for PORT in 8001 8002; do
    if lsof -i:"$PORT" >/dev/null 2>&1; then
        print_color $YELLOW "🔧 Killing process on port $PORT..."
        lsof -ti:"$PORT" | xargs -r kill -9 || true
    fi
done

# Check if backend is accessible (with timeout)
if ! timeout 5 curl -s http://localhost:8001/ > /dev/null; then
    print_color $YELLOW "⚠️  Backend not responding at http://localhost:8001/"
    print_color $YELLOW "   The test runner will start the backend automatically."
fi

print_color $GREEN "✅ Pre-flight checks completed"
echo ""

# Setup test environment
print_color $YELLOW "🛠️  Setting up test environment..."

# Create cache and temp directories
export PLAYWRIGHT_CACHE_DIR="${SCRIPT_DIR}/.pw_cache"
mkdir -p "$PLAYWRIGHT_CACHE_DIR"
export TMPDIR="${SCRIPT_DIR}/.pw_tmp"
mkdir -p "$TMPDIR"
export PW_TMP_DIR="$TMPDIR"

# Set environment variables
export NODE_ENV=test
export E2E_LOG_SUPPRESS=1

print_color $GREEN "✅ Environment setup completed"
echo ""

# Get test files for the selected mode
TEST_FILES=${TEST_CATEGORIES[$MODE]}

print_color $YELLOW "🧪 Running $MODE tests..."
print_color $CYAN "Test files: $TEST_FILES"
echo ""

# Build Playwright command based on mode and options
PLAYWRIGHT_CMD="npx playwright test"

# Add test file filters
if [[ -n "$TEST_FILES" ]]; then
    for file in $TEST_FILES; do
        PLAYWRIGHT_CMD="$PLAYWRIGHT_CMD tests/$file"
    done
fi

# Add reporter based on mode
case $MODE in
    basic)
        PLAYWRIGHT_CMD="$PLAYWRIGHT_CMD --reporter=line"
        ;;
    full)
        PLAYWRIGHT_CMD="$PLAYWRIGHT_CMD --reporter=html --reporter=json"
        ;;
esac

# Add parallel execution unless disabled
if [[ "$PARALLEL" == "true" ]]; then
    PLAYWRIGHT_CMD="$PLAYWRIGHT_CMD --workers=2"
else
    PLAYWRIGHT_CMD="$PLAYWRIGHT_CMD --workers=1"
fi

# Add verbose output if requested
if [[ "$VERBOSE" == "true" ]]; then
    PLAYWRIGHT_CMD="$PLAYWRIGHT_CMD --verbose"
fi

# Add timeout based on mode
case $MODE in
    basic)
        PLAYWRIGHT_CMD="$PLAYWRIGHT_CMD --timeout=30000"
        ;;
    full)
        PLAYWRIGHT_CMD="$PLAYWRIGHT_CMD --timeout=60000"
        ;;
esac

# Execute the tests
print_color $BLUE "🚀 Executing: $PLAYWRIGHT_CMD"
echo ""

START_TIME=$(date +%s)

# Run the tests and capture exit code
set +e
eval $PLAYWRIGHT_CMD
TEST_EXIT_CODE=$?
set -e

END_TIME=$(date +%s)
DURATION=$((END_TIME - START_TIME))

echo ""
print_color $BLUE "📊 Test Execution Summary"
print_color $BLUE "========================"
echo "Mode: $MODE"
echo "Duration: ${DURATION}s"
echo "Exit Code: $TEST_EXIT_CODE"

# Generate mode-specific summary
case $MODE in
    basic)
        if [[ $TEST_EXIT_CODE -eq 0 ]]; then
            print_color $GREEN "✅ Basic tests passed - core functionality working"
        else
            print_color $RED "❌ Basic tests failed - check core functionality"
        fi
        ;;
    full)
        if [[ $TEST_EXIT_CODE -eq 0 ]]; then
            print_color $GREEN "🎉 Full test suite passed - this is indeed the best tested app!"
        else
            print_color $RED "❌ Full test suite failed - check detailed reports"
        fi
        ;;
esac

# Clean up test databases
print_color $YELLOW "🧹 Cleaning up test databases..."
if [ -f "../backend/cleanup_test_dbs.sh" ]; then
    (cd ../backend && ./cleanup_test_dbs.sh)
fi

# Clean up temp files for basic mode
if [[ "$MODE" == "basic" ]]; then
    rm -rf "$TMPDIR" 2>/dev/null || true
fi

print_color $GREEN "✅ Cleanup completed"
echo ""

# Final status
if [[ $TEST_EXIT_CODE -eq 0 ]]; then
    print_color $GREEN "🎯 E2E Test Suite Completed Successfully"
else
    print_color $RED "💥 E2E Test Suite Failed"
fi

exit $TEST_EXIT_CODE