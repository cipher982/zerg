# Testing Guide

This document outlines the testing structure and how to run tests for the Zerg Agent Platform.

## Test Structure

The project now has a clean, organized testing structure:

```
/
├── backend/
│   ├── tests/                    # Backend unit/integration tests
│   └── run_backend_tests.sh      # Backend test runner
├── frontend/
│   ├── src/tests/                # Frontend unit tests (WASM)
│   └── run_frontend_tests.sh     # Frontend test runner
├── e2e/                          # End-to-end tests (NEW)
│   ├── tests/
│   │   ├── dashboard.basic.spec.js
│   │   ├── dashboard.scope-toggle.spec.js
│   │   └── modal_tab_visibility.spec.ts
│   ├── playwright.config.js
│   ├── package.json
│   ├── run_e2e_tests.sh
│   └── README.md
├── prerender/                    # Pure SEO prerendering (cleaned up)
│   ├── prerender.js
│   ├── server.js
│   └── package.json (no test deps)
└── scripts/                      # Root-level test orchestration
    └── run_all_tests.sh
```

## Running Tests

### All Tests (Recommended)
```bash
./scripts/run_all_tests.sh
```
This runs backend + frontend + E2E tests with a summary report.

### Individual Test Suites

**Backend Tests:**
```bash
cd backend && ./run_backend_tests.sh
```
- >95% test coverage
- Uses in-memory SQLite
- No external dependencies needed

**Frontend Tests:**
```bash
cd frontend && ./run_frontend_tests.sh
```
- WASM unit tests using wasm-bindgen-test
- Requires Chrome or Firefox browser

**E2E Tests:**
```bash
cd e2e && ./run_e2e_tests.sh
```
- Playwright-based end-to-end tests
- Automatically starts backend and frontend servers
- Tests dashboard functionality and UI interactions

## Recent Changes

**What was reorganized:**
- Consolidated E2E tests from `prerender/tests/` and `frontend/e2e/` into unified `e2e/` directory
- Removed Playwright dependencies from `prerender/package.json` 
- Created master test orchestration script
- Clean separation of concerns: prerender is now purely for SEO

**Benefits:**
- Clear separation of test types
- Consistent test runner patterns
- Easy to run all tests or individual suites
- Scalable structure for future test additions

## Test Categories

- **Backend**: Unit and integration tests for Python/FastAPI code
- **Frontend**: WASM unit tests for Rust frontend code  
- **E2E**: Full-stack browser tests using Playwright
- **Prerender**: SEO functionality (separate from testing)

For detailed E2E test information, see `e2e/README.md`.
