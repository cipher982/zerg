# End-to-End Tests

This directory contains the consolidated E2E test suite for the Zerg Agent Platform using Playwright.

## Structure

```
e2e/
├── tests/                          # Test specifications
│   ├── dashboard.basic.spec.js     # Basic dashboard functionality
│   ├── dashboard.scope-toggle.spec.js  # Dashboard scope selector
│   └── modal_tab_visibility.spec.ts    # Modal tab behavior
├── playwright.config.js           # Playwright configuration
├── package.json                   # Dependencies and scripts
├── run_e2e_tests.sh               # Test runner script
└── README.md                      # This file
```

## Running Tests

### Quick Start
```bash
# From the e2e directory
./run_e2e_tests.sh
```

### Manual Commands
```bash
# Install dependencies (if needed)
npm install

# Run all tests
npm test

# Run tests with browser visible (headed mode)
npm run test:headed

# Debug tests interactively
npm run test:debug
```

### From Root Directory
```bash
# Run all test suites (backend + frontend + e2e)
./scripts/run_all_tests.sh

# Run just E2E tests
./e2e/run_e2e_tests.sh
```

## Configuration

The Playwright configuration automatically:
- Starts the backend server (port 8001) if not already running
- Starts the frontend dev server (port 8002) if not already running
- Runs tests in headless mode by default
- Uses a 1280x800 viewport
- Retries failed tests in CI environments

## Adding New Tests

1. Create a new `.spec.js` or `.spec.ts` file in the `tests/` directory
2. Follow the existing patterns for test structure
3. Use the base URL `http://localhost:8002` (configured automatically)
4. Tests will be discovered and run automatically

## Test Categories

- **Dashboard tests**: Verify the main dashboard functionality
- **Modal tests**: Test modal dialogs and their behavior
- **Navigation tests**: Verify routing and navigation flows

## Dependencies

- `@playwright/test`: Core testing framework
- Automatically managed browser binaries via Playwright

## Notes

- Tests assume the backend and frontend are available on their default ports
- The test runner will start servers automatically if they're not running
- All tests run in parallel by default for faster execution
- Failed tests generate traces for debugging (on first retry)
