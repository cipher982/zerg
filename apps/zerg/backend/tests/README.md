# Test Suite Documentation

## Overview

This directory contains the test suite for the Zerg backend application. Tests are organized by type to provide clear separation of concerns and fast feedback during development.

## Test Structure

```
tests/
├── unit/              # Fast, isolated unit tests (no I/O, no database)
├── integration/       # Integration tests (API endpoints, services with DB)
├── e2e/              # End-to-end workflow tests
├── contracts/        # Contract validation tests
├── helpers/          # Shared test utilities
├── conftest_parts/   # Conftest organization (future use)
├── conftest.py       # Main pytest configuration and fixtures
└── README.md         # This file
```

## Running Tests

### All Tests
```bash
pytest
```

### By Category
```bash
# Unit tests (fast, < 30s)
pytest tests/unit/ -v

# Integration tests
pytest tests/integration/ -v

# E2E tests
pytest tests/e2e/ -v

# Contract tests
pytest tests/contracts/ -v
```

### With Coverage
```bash
pytest --cov=zerg --cov-report=html --cov-report=term-missing
```

### Parallel Execution
```bash
pytest -n auto  # Use all CPU cores
```

### Watch Mode (requires pytest-watch)
```bash
ptw  # Re-runs tests on file changes
```

## Test Configuration

### pytest.ini
Located in `apps/zerg/backend/pytest.ini`:
- Sets pythonpath for proper imports
- Configures test discovery paths
- Defines test markers
- Sets async test defaults
- Configures timeouts (10s default)

### conftest.py
Main test configuration file containing:
- **Database Setup**: PostgreSQL via Testcontainers
- **Mock Configuration**: OpenAI, LangSmith, LangChain stubs
- **Fixtures**: Database sessions, test clients, sample data
- **Cleanup**: Session-level cleanup for global resources

## Writing Tests

### Test Naming Convention

```python
def test_<feature>_<scenario>_<expected_result>():
    """Test that <feature> <expected_result> when <scenario>."""
    pass
```

Examples:
- `test_create_agent_returns_201_with_valid_data()`
- `test_create_agent_returns_422_when_model_invalid()`
- `test_workflow_execution_completes_with_empty_canvas()`

### Test Structure (Arrange-Act-Assert)

```python
def test_example(client, db_session):
    """Test description."""
    # Arrange - Set up test data
    user = create_test_user(db_session)
    agent_data = {"name": "Test", ...}
    
    # Act - Perform the action
    response = client.post("/api/agents", json=agent_data)
    
    # Assert - Verify the result
    assert response.status_code == 201
    assert response.json()["name"] == agent_data["name"]
```

### Unit Test Guidelines

**DO:**
- Test pure functions with no side effects
- Mock external dependencies
- Keep tests fast (< 0.1s each)
- Test one thing per test
- Use descriptive test names

**DON'T:**
- Access databases or external services
- Depend on test execution order
- Test implementation details
- Share state between tests

### Integration Test Guidelines

**DO:**
- Use test database (via fixtures)
- Test API endpoints end-to-end
- Verify database state changes
- Test service interactions
- Clean up after tests

**DON'T:**
- Call external APIs (mock them)
- Leave data in database
- Make tests dependent on each other

### E2E Test Guidelines

**DO:**
- Test complete user workflows
- Verify end-to-end functionality
- Test critical business paths
- Include setup and teardown

**DON'T:**
- Test every edge case (use unit tests)
- Make tests too long (split into scenarios)
- Skip cleanup

## Common Fixtures

### Database Fixtures

```python
def test_with_database(db_session):
    """db_session - Clean database for each test."""
    user = User(email="test@example.com")
    db_session.add(user)
    db_session.commit()
    # Database automatically cleaned up after test
```

### HTTP Client Fixtures

```python
def test_authenticated_endpoint(client, auth_headers):
    """client - FastAPI TestClient with auth."""
    response = client.get("/api/agents", headers=auth_headers)
    assert response.status_code == 200
```

```python
def test_unauthenticated_endpoint(unauthenticated_client):
    """Test without authentication."""
    response = unauthenticated_client.get("/api/agents")
    # In test mode, auth is disabled, so this still works
```

### Sample Data Fixtures

```python
def test_with_agent(sample_agent, db_session):
    """sample_agent - Pre-created agent."""
    assert sample_agent.id is not None
    assert sample_agent.name == "Test Agent"
```

Available sample fixtures:
- `sample_agent` - Test agent
- `sample_messages` - Agent messages
- `sample_thread` - Test thread
- `sample_thread_messages` - Thread messages
- `_dev_user` / `test_user` - Test user
- `other_user` - Second user for isolation tests

## Test Markers

### Async Tests
```python
@pytest.mark.asyncio
async def test_async_function():
    result = await some_async_function()
    assert result is not None
```

### Database Isolation
```python
@pytest.mark.db_isolation
def test_database_isolation(db_session):
    """Tests that verify database isolation."""
    pass
```

### First Priority
```python
@pytest.mark.first
def test_early_detection():
    """Tests that should run first to detect issues early."""
    pass
```

## Debugging Tests

### Run Specific Test
```bash
pytest tests/integration/test_agents.py::test_create_agent -v
```

### Print Output
```bash
pytest tests/integration/test_agents.py -s  # Show print statements
```

### Debug with PDB
```bash
pytest tests/integration/test_agents.py --pdb  # Drop into debugger on failure
```

### Verbose Output
```bash
pytest -vv  # Extra verbose
```

## Common Issues

### Import Errors
If you see import errors, ensure you're running pytest from the backend directory:
```bash
cd apps/zerg/backend
pytest
```

### Database Connection Errors
Tests use Testcontainers which requires Docker:
```bash
# Check Docker is running
docker ps

# If on macOS, ensure Docker Desktop is running
```

### Timeout Errors
Default timeout is 10s (configured in pytest.ini). For slow tests:
```python
@pytest.mark.timeout(30)  # Override timeout
def test_slow_operation():
    pass
```

### Fixture Not Found
Ensure you're importing from the correct location:
- Core fixtures are in `conftest.py`
- Helper functions are in `helpers/`

## Test Maintenance

### Before Committing
```bash
# Run all tests
pytest

# Check coverage
pytest --cov=zerg --cov-report=term-missing

# Run linter
ruff check .
ruff format .
```

### Adding New Tests
1. Choose appropriate directory (unit/integration/e2e)
2. Follow naming conventions
3. Use existing fixtures when possible
4. Add docstrings
5. Ensure test is independent
6. Verify it passes

### Removing Tests
Only remove tests if:
- Feature no longer exists
- Test is duplicated
- Test is fundamentally broken and unfixable

Document removal in commit message.

## Performance

### Current Test Suite Performance
- **Unit Tests**: ~20s (when we have them)
- **Integration Tests**: ~2-5min
- **E2E Tests**: ~5-10min
- **Total**: ~5-15min (depending on hardware)

### Optimization Tips
1. Use fixtures efficiently (scope appropriately)
2. Mock external services
3. Run in parallel with `pytest -n auto`
4. Keep unit tests truly isolated

## Contributing

### Test Review Checklist
- [ ] Test has clear, descriptive name
- [ ] Test has docstring explaining what it tests
- [ ] Test follows Arrange-Act-Assert pattern
- [ ] Test is in correct directory (unit/integration/e2e)
- [ ] Test uses appropriate fixtures
- [ ] Test cleans up after itself
- [ ] Test passes consistently
- [ ] Test is fast (unit < 0.1s, integration < 1s)

## Resources

- [Pytest Documentation](https://docs.pytest.org/)
- [FastAPI Testing](https://fastapi.tiangolo.com/tutorial/testing/)
- [Testcontainers Python](https://testcontainers-python.readthedocs.io/)

## Recent Changes

### 2025-10-11 - Test Suite Cleanup
- Removed broken/skipped WebSocket tests
- Deleted unused test files (test_simple.py, debug_test.py)
- Moved misplaced root-level tests to proper locations
- Removed test_pact_contracts.py (missing dependencies)
- Removed test_workflow_http_integration.py (broken async handling)
- Commented out skipped tests in workflow_execution_cancel and workflow_scheduling
- Created new test directory structure (unit/, integration/, e2e/, contracts/)
- Improved test organization

### Test Statistics (Post-Cleanup)
- **Total Test Files**: ~79 (down from 83+)
- **Skipped Tests**: 0 (down from ~30)
- **Test Functions**: ~350+
- **Coverage**: TBD (run `pytest --cov` to check)

## Contact

For questions about tests, see:
- Main docs: `TESTING_DEEP_DIVE_REPORT.md` in repo root
- Quick actions: `TESTING_IMMEDIATE_ACTIONS.md` in repo root
