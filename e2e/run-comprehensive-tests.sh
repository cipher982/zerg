#!/bin/bash

# COMPREHENSIVE E2E TEST RUNNER
# 
# This script runs the complete E2E test suite with proper configuration
# and generates comprehensive reports for all test categories.

set -e

echo "🚀 Starting Comprehensive E2E Test Suite"
echo "========================================"

# Configuration
TEST_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPORTS_DIR="$TEST_DIR/test-reports"
TIMESTAMP=$(date +"%Y%m%d_%H%M%S")

# Create reports directory
mkdir -p "$REPORTS_DIR"

# Test categories and their descriptions
declare -A TEST_CATEGORIES
TEST_CATEGORIES[agent_creation_full]="Agent Creation and Management"
TEST_CATEGORIES[comprehensive_debug]="System Debug and Diagnostics"
TEST_CATEGORIES[canvas_complete_workflow]="Canvas Workflow Operations"
TEST_CATEGORIES[workflow_execution_http]="HTTP Tool Integration"
TEST_CATEGORIES[realtime_websocket_monitoring]="WebSocket Real-time Monitoring"
TEST_CATEGORIES[error_handling_edge_cases]="Error Handling and Edge Cases"
TEST_CATEGORIES[data_persistence_recovery]="Data Persistence and Recovery"
TEST_CATEGORIES[tool_palette_node_connections]="Tool Palette and Node Connections"
TEST_CATEGORIES[performance_load_testing]="Performance and Load Testing"
TEST_CATEGORIES[accessibility_ui_ux]="Accessibility and UI/UX"
TEST_CATEGORIES[multi_user_concurrency]="Multi-user and Concurrency"

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

# Function to run a single test category
run_test_category() {
    local test_file=$1
    local test_name=$2
    local description=$3
    
    print_color $BLUE "📊 Running: $description"
    print_color $CYAN "   File: $test_file"
    echo "   Started: $(date)"
    
    local start_time=$(date +%s)
    local success=true
    
    # Run the test with proper configuration
    if npx playwright test "tests/$test_file" \
        --config=playwright.config.ts \
        --reporter=json \
        --output="$REPORTS_DIR/${test_name}_${TIMESTAMP}" \
        > "$REPORTS_DIR/${test_name}_${TIMESTAMP}.log" 2>&1; then
        
        local end_time=$(date +%s)
        local duration=$((end_time - start_time))
        
        print_color $GREEN "✅ PASSED: $description (${duration}s)"
    else
        local end_time=$(date +%s)
        local duration=$((end_time - start_time))
        
        print_color $RED "❌ FAILED: $description (${duration}s)"
        success=false
    fi
    
    echo "   Completed: $(date)"
    echo "   Log: $REPORTS_DIR/${test_name}_${TIMESTAMP}.log"
    echo ""
    
    return $([ "$success" = true ] && echo 0 || echo 1)
}

# Function to generate summary report
generate_summary() {
    local total_tests=$1
    local passed_tests=$2
    local failed_tests=$3
    local duration=$4
    
    local summary_file="$REPORTS_DIR/test_summary_${TIMESTAMP}.md"
    
    cat > "$summary_file" << EOF
# Comprehensive E2E Test Suite Summary

**Execution Date:** $(date)
**Total Duration:** ${duration} seconds
**Total Test Categories:** $total_tests
**Passed:** $passed_tests
**Failed:** $failed_tests
**Success Rate:** $(( (passed_tests * 100) / total_tests ))%

## Test Categories Overview

EOF

    for test_name in "${!TEST_CATEGORIES[@]}"; do
        local description="${TEST_CATEGORIES[$test_name]}"
        local log_file="$REPORTS_DIR/${test_name}_${TIMESTAMP}.log"
        
        if [ -f "$log_file" ]; then
            if grep -q "failed" "$log_file" || grep -q "error" "$log_file"; then
                echo "- ❌ **$description** - FAILED" >> "$summary_file"
            else
                echo "- ✅ **$description** - PASSED" >> "$summary_file"
            fi
        else
            echo "- ⚠️  **$description** - NOT RUN" >> "$summary_file"
        fi
    done
    
    cat >> "$summary_file" << EOF

## Test Coverage Areas

### 🔧 Core Functionality
- Agent creation and management
- Workflow execution and monitoring
- Canvas operations and node connections
- Tool palette integration

### 🛡️ Reliability & Quality
- Error handling and edge cases
- Data persistence and recovery
- Performance and load testing
- Multi-user concurrency handling

### ♿ Accessibility & UX
- WCAG compliance testing
- Keyboard navigation
- Screen reader compatibility
- Responsive design validation

### 🔍 Monitoring & Debugging
- Real-time WebSocket monitoring
- System diagnostics and debugging
- Database isolation validation
- Session management testing

## Detailed Logs

All detailed test logs are available in: \`$REPORTS_DIR/\`

EOF

    print_color $PURPLE "📋 Summary report generated: $summary_file"
}

# Main execution
print_color $YELLOW "🔧 Pre-flight checks..."

# Check if Playwright is installed
if ! command -v npx &> /dev/null; then
    print_color $RED "❌ npx not found. Please install Node.js and npm."
    exit 1
fi

# Check if Playwright tests can run
if [ ! -f "$TEST_DIR/playwright.config.ts" ]; then
    print_color $RED "❌ Playwright config not found at $TEST_DIR/playwright.config.ts"
    exit 1
fi

# Check if backend is running
if ! curl -s http://localhost:8001/ > /dev/null; then
    print_color $YELLOW "⚠️  Backend not responding at http://localhost:8001/"
    print_color $YELLOW "   Please ensure the backend is running before starting tests."
    read -p "   Continue anyway? (y/N): " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 1
    fi
fi

print_color $GREEN "✅ Pre-flight checks completed"
echo ""

# Start test execution
print_color $YELLOW "🚀 Starting comprehensive test execution..."
echo ""

total_tests=0
passed_tests=0
failed_tests=0
start_time=$(date +%s)

# Run each test category
for test_name in $(echo "${!TEST_CATEGORIES[@]}" | tr ' ' '\n' | sort); do
    description="${TEST_CATEGORIES[$test_name]}"
    test_file="${test_name}.spec.ts"
    
    total_tests=$((total_tests + 1))
    
    if run_test_category "$test_file" "$test_name" "$description"; then
        passed_tests=$((passed_tests + 1))
    else
        failed_tests=$((failed_tests + 1))
    fi
done

# Calculate total duration
end_time=$(date +%s)
total_duration=$((end_time - start_time))

# Generate summary
echo ""
print_color $YELLOW "📊 Generating test summary..."
generate_summary $total_tests $passed_tests $failed_tests $total_duration

# Final results
echo ""
print_color $YELLOW "🏁 Test Execution Complete"
print_color $YELLOW "=========================="
echo "Total Tests: $total_tests"
print_color $GREEN "Passed: $passed_tests"
print_color $RED "Failed: $failed_tests"
echo "Duration: $total_duration seconds"
echo "Reports: $REPORTS_DIR"

if [ $failed_tests -eq 0 ]; then
    print_color $GREEN "🎉 ALL TESTS PASSED! This is indeed the best tested app ever!"
    exit 0
else
    print_color $YELLOW "⚠️  Some tests failed. Check the logs for details."
    exit 1
fi