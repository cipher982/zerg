# Testing Suite - Immediate Actions

**TL;DR:** Your test suite has 366 tests across 141 files, but ~30 tests are skipped/broken. Tests are scattered across multiple locations with no clear organization. This document provides actionable steps to fix critical issues.

---

## 🚨 Critical Issues Found

1. **Entire WebSocket test suites disabled** (30+ tests) - No real-time feature validation
2. **Workflow tests broken after LangGraph migration** - Core features untested  
3. **Tests scattered in wrong locations** - 13 files at root/script level instead of tests/
4. **Complex test infrastructure** - 742-line conftest.py making tests hard to run
5. **Multiple redundant contract validation approaches** - Wasted effort, none comprehensive

---

## ✅ Quick Wins (Do Today - < 1 Hour)

### 1. Delete Obviously Broken/Useless Tests

```bash
cd /workspace/apps/zerg/backend

# Remove no-op test
rm test_simple.py

# Remove debug script (not a test)
rm /workspace/debug_test.py

# Remove obsolete cleanup script
rm cleanup_test_dbs.py
```

### 2. Document Why Tests Are Skipped

Add GitHub issue numbers to skipped tests:

```python
# Before
@pytest.mark.skip(reason="Temporarily disabled due to hangs and logging issues")

# After
@pytest.mark.skip(reason="TODO: Fix WebSocket test hangs - Track in issue #XXX")
```

Files to update:
- `tests/test_websocket.py`
- `tests/test_websocket_integration.py`
- `tests/test_workflow_execution_cancel.py`
- `tests/test_workflow_scheduling.py`

### 3. Move Misplaced Root-Level Tests

```bash
cd /workspace

# Create proper structure
mkdir -p apps/zerg/backend/tests/integration
mkdir -p apps/zerg/backend/tests/contracts

# Move tests to proper locations
mv test_reserve_api.py apps/zerg/backend/tests/integration/
mv test_websocket_subscription.py apps/zerg/backend/tests/integration/
mv test_schema_routing.py apps/zerg/backend/tests/contracts/
mv tests/test_ws_protocol_contracts.py apps/zerg/backend/tests/contracts/

# Fix imports in moved files (they may reference wrong paths)
```

**Estimated Time:** 30-45 minutes  
**Risk:** Low (files are standalone)  
**Impact:** High (cleaner structure)

---

## 📋 This Week Actions (< 1 Day)

### 1. Fix One Skipped Test Suite

**Start with smallest:** `test_workflow_execution_cancel.py` (1 skipped test)

**Options:**
- A) Rewrite for LangGraph (2-4 hours)
- B) Delete if cancellation not used (5 minutes)
- C) Create issue and schedule for later (10 minutes)

**Recommended:** Option C for now, fix later

### 2. Split conftest.py Into Focused Files

```bash
cd /workspace/apps/zerg/backend/tests

# Create new conftest directory
mkdir conftest_parts

# Split into focused files
# 1. conftest_parts/fixtures.py - test data fixtures
# 2. conftest_parts/mocks.py - mock configurations  
# 3. conftest_parts/containers.py - Docker/testcontainers setup
# 4. Keep main conftest.py for core only
```

**Goal:** Reduce main conftest.py from 742 lines to < 200 lines

**Estimated Time:** 3-4 hours  
**Risk:** Medium (could break tests if imports wrong)  
**Impact:** High (easier to understand and maintain)

### 3. Consolidate Contract Testing

**Decision:** Pick ONE contract testing approach and delete the rest.

**Current approaches:**
1. ✅ **KEEP:** `test_tool_contracts.py` - Working tool validation
2. ✅ **KEEP:** `test_api_contract_canvas.py` - Working API validation  
3. ❌ **DELETE:** `test_pact_contracts.py` - Requires missing dependency
4. ❌ **CONSOLIDATE:** Shell scripts → Move into pytest

**Action:**
```bash
# Delete broken pact tests
rm apps/zerg/backend/tests/test_pact_contracts.py

# Consolidate scripts into pytest
# Move scripts/validate_tool_contracts.py → tests/contracts/
# Convert shell scripts to pytest functions
```

**Estimated Time:** 2-3 hours  
**Risk:** Low (removing broken code)  
**Impact:** Medium (less confusion)

---

## 📅 This Month Actions (< 1 Week)

### 1. Fix All Skipped Tests (Priority 1)

**Files to fix:**
- `test_websocket.py` - 8 tests skipped
- `test_websocket_integration.py` - 2 tests skipped
- `test_workflow_execution_cancel.py` - 1 test skipped
- `test_workflow_http_integration.py` - 2 tests skipped
- `test_workflow_scheduling.py` - 1 test skipped
- `test_pact_contracts.py` - Delete (no dependency)

**Decision Matrix:**
```
For each skipped test:
├─ Can fix in < 2 hours?
│  ├─ YES → Fix it
│  └─ NO → Is feature actively used?
│     ├─ YES → Create ticket, schedule fix
│     └─ NO → Delete test
```

**Estimated Time:** 1-2 days  
**Success Metric:** Zero skipped tests in main branch

### 2. Reorganize Test Structure (Priority 2)

**Create new structure:**
```
tests/
├── unit/              ← Fast, isolated, no I/O
├── integration/       ← API/service integration  
├── e2e/              ← Full workflow tests
├── contracts/        ← Contract validation
└── conftest/         ← Split configuration
```

**Migration plan:**
1. Create new directories
2. Classify existing tests (unit vs integration vs e2e)
3. Move tests in batches of 10
4. Run tests after each batch
5. Update imports

**Estimated Time:** 2-3 days  
**Risk:** Medium (could break imports)  
**Impact:** High (much clearer structure)

### 3. Update CI/CD Pipeline (Priority 3)

**Update `.github/workflows/` to:**
1. Run unit tests first (fast feedback)
2. Run integration tests second
3. Run e2e tests last (slow but comprehensive)
4. Fail fast on unit test failures
5. Report coverage

**Example workflow:**
```yaml
- name: Unit Tests
  run: pytest tests/unit/ -v --cov
  timeout-minutes: 2

- name: Integration Tests  
  run: pytest tests/integration/ -v
  timeout-minutes: 5
  
- name: E2E Tests
  run: pytest tests/e2e/ -v
  timeout-minutes: 10
```

**Estimated Time:** 4-6 hours  
**Impact:** High (better developer experience)

---

## 📊 Progress Tracking

### Metrics to Track

**Before Cleanup:**
- Tests: 366 functions
- Skipped: ~30 tests (8%)
- Files: 141 total
- Misplaced: 13 files
- Organization: 1/10

**Target After Cleanup:**
- Tests: 330-350 functions (removed broken/duplicate)
- Skipped: 0 tests (0%)
- Files: ~100 total (consolidated)
- Misplaced: 0 files
- Organization: 8/10

### Weekly Checklist

**Week 1:**
- [ ] Delete useless tests (test_simple.py, debug_test.py)
- [ ] Move misplaced root-level tests
- [ ] Document skipped tests with issue numbers
- [ ] Fix or delete 1 skipped test suite

**Week 2:**
- [ ] Split conftest.py into focused files
- [ ] Consolidate contract testing approach
- [ ] Create new test directory structure
- [ ] Classify 50% of existing tests

**Week 3:**
- [ ] Migrate all tests to new structure
- [ ] Fix remaining skipped tests
- [ ] Update CI/CD pipeline
- [ ] Add test documentation (README)

**Week 4:**
- [ ] Run full test suite and verify
- [ ] Measure coverage
- [ ] Create test templates
- [ ] Add pre-commit hooks

---

## 🎯 Success Criteria

You'll know you're done when:

1. ✅ **Zero skipped tests** - All tests pass or are deleted
2. ✅ **Clean structure** - Tests in proper directories
3. ✅ **Fast feedback** - Unit tests run in < 30 seconds
4. ✅ **Clear docs** - README explains test organization
5. ✅ **Automated** - CI enforces test quality
6. ✅ **Maintainable** - New developers can understand tests

---

## 🚀 Getting Started

**Right Now:**
```bash
# 1. Create a branch for cleanup
git checkout -b test-suite-cleanup

# 2. Do quick wins (30 minutes)
rm apps/zerg/backend/test_simple.py
rm debug_test.py

# 3. Move root-level tests
mkdir -p apps/zerg/backend/tests/{integration,contracts}
mv test_reserve_api.py apps/zerg/backend/tests/integration/
# ... (continue with other moves)

# 4. Commit and push
git add .
git commit -m "test: cleanup - remove useless tests and move misplaced files"
git push origin test-suite-cleanup
```

**Need Help?** Ask me to:
- Fix a specific skipped test
- Move and update imports for misplaced tests
- Split conftest.py
- Create the new directory structure
- Update CI/CD workflows

---

*Generated: 2025-10-11*  
*See TESTING_DEEP_DIVE_REPORT.md for full analysis*
