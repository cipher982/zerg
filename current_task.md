**Comprehensive Research & Targeted Report: Database/Session/Route Refactor**

---

### 1. **Current State: What We Found (BEFORE)**

#### **Database Setup**
- `backend/zerg/app/database.py` used to create a global `engine` and `SessionLocal` at import time, using an env-var or default SQLite file.
- `get_db()` yielded a session from `SessionLocal`.
- `Base.metadata.create_all(bind=engine)` was called in multiple places: app startup, admin router, and a legacy `init_db.py` script.

#### **Test Setup**
- `backend/tests/conftest.py` created a separate in-memory engine and session for tests, monkey-patching `zerg.app.database.engine` and `SessionLocal`.
- Test fixtures overrode `get_db` via FastAPI's `dependency_overrides`.
- Tables were created/dropped per test via `Base.metadata.create_all/drop_all`.

#### **App Startup & Routers**
- `main.py` included routers under `/api` and created tables on startup.
- There was a `/api/reset-database` endpoint and a `/admin/reset-database` endpoint, both for dev use.
- Some routers (e.g., WebSocket, scheduler) used `SessionLocal` directly, not via dependency injection.

#### **Model/CRUD Layer**
- All CRUD functions took a `db: Session` argument, so were decoupled from session creation.
- Models were clean, using `Base` from `database.py`.

#### **Redundant/Legacy Code**
- `init_db.py` was a legacy script for table creation.
- Multiple places could create/drop tables, risking confusion.

---

### 2. **Problems Identified (BEFORE)**

- **Global State:** `engine` and `SessionLocal` were global, patched in tests, and used directly in some services/routers.
- **Multiple Table Creation Points:** Tables were created in app startup, admin router, and `init_db.py`.
- **Direct Session Usage:** Some code (WebSocket, scheduler) used `SessionLocal` directly, bypassing DI.
- **Test Complexity:** Tests monkey-patched globals and overrode dependencies, which was fragile.
- **Redundant Endpoints:** Multiple DB reset endpoints, some not secured.

---

### 3. **Best Practice Goals**

- **Single Source of Truth:** Only one place to configure/create the engine/session.
- **Dependency Injection:** All DB access via DI, never direct use of `SessionLocal`.
- **Centralized Table Management:** Only create/drop tables in app startup and test setup.
- **Test Simplicity:** No monkey-patching; just override dependencies.
- **Remove Redundancy:** Eliminate legacy scripts and duplicate endpoints.

---

### 4. **Targeted Refactor Plan**

#### **A. Refactor Database Setup**
- [x] Move engine/session creation into factory functions:
  ```python
  def make_engine(db_url): ...
  def make_sessionmaker(engine): ...
  ```
- [x] Expose a `get_session_factory()` that uses the current settings.

#### **B. Dependency Injection Everywhere**
- [x] Change all code (WebSocket, scheduler, etc.) to accept a session or session factory via DI, not direct import.
- [x] `get_db()` should accept a session factory, defaulting to the app's.

#### **C. Centralize Table Creation**
- [x] Only call `Base.metadata.create_all()` in:
  - App startup event (prod/dev)
  - Test fixture setup (tests)
- [x] Remove `init_db.py` and admin DB reset endpoints if not needed.

#### **D. Simplify Tests**
- [x] Use a single `db_session` fixture that:
  - Builds an in-memory engine/session factory
  - Creates/drops tables per test/session
- [x] Override `get_db` via `app.dependency_overrides` only.

#### **E. Remove Redundant Code**
- [x] Delete `init_db.py` and any admin endpoints for DB reset if not needed for prod.
- [x] Ensure all table management is in startup/test setup.

---

### 5. **Concrete Next Steps**

**COMPLETED:**
1. **Draft a new `database.py` with factories and no global state.**
2. **Update all usages of `SessionLocal` to use DI.**
3. **Remove `init_db.py` and duplicate DB reset endpoints.**
4. **Refactor test setup to use only dependency overrides, no monkey-patching.**
5. **Document the new pattern in the README or a dev guide.**

---

### 6. **Risks & Mitigations**

- **Risk:** Refactor may break tests or runtime if any code still uses globals.
  - **Mitigation:** Searched for all `SessionLocal`/`engine` usages and updated.
- **Risk:** Table creation may be missed in some environments.
  - **Mitigation:** Ensured startup/test setup is the only place for this logic.

---

### 7. **Summary Table (NOW)**

| Area                | Current State         | Target State         | Action Needed         |
|---------------------|----------------------|----------------------|----------------------|
| Engine/Session      | Factory, DI          | Factory, DI          | **DONE**             |
| Table Creation      | Startup/test only    | Startup/test only    | **DONE**             |
| Session Usage       | Always via DI        | Always via DI        | **DONE**             |
| Test Setup          | Only override        | Only override        | **DONE**             |
| Redundant Endpoints | None or one, secured | None or one, secured | **DONE**             |

---

### 8. **What We Learned / Best Practices**

- Use factory functions for engine/session creation for flexibility and testability.
- Always use dependency injection for DB access (never import global session/engine).
- Centralize table creation in app startup and test setup only.
- Tests should use dependency overrides, not monkey-patching.
- Remove legacy scripts and redundant endpoints for clarity.
- Document the pattern for future contributors (see `backend/DATABASE.md`).

---

### 9. **What Remains / Future TODOs**

- [x] Review for any remaining direct `SessionLocal`/`engine` usage in new code or future PRs.
- [x] Consider removing the `/admin/reset-database` endpoint in production or securing it if needed.
  - Added an environment check to prevent usage in non-development environments
- [x] Continue to enforce these patterns in code review and onboarding.
  - Updated DATABASE.md with clear documentation and examples
- [ ] (Optional) Add automated lint/test to check for direct session/engine usage.
- [ ] (Optional) Add more documentation/examples for contributors.

---

**All major refactor goals are complete. The codebase is now modular, testable, and follows best practices for DB/session management.**

### 10. **Completed Cleanup Tasks**

1. **Removed redundant database reset endpoint** 
   - Eliminated `/api/reset-database` from main.py, keeping only the admin endpoint

2. **Added environment-based security check**
   - Reset endpoint now checks for `ENVIRONMENT=development` before allowing database resets
   - Updated .env.example to document this requirement

3. **Improved WebSocket session handling**
   - Created explicit `get_websocket_session()` function to make the pattern more obvious
   - Consistent session management pattern across all WebSocket handlers

4. **Enhanced documentation**
   - Updated DATABASE.md with examples for all common patterns
   - Added section about the database reset endpoint and its security constraints

The refactor is now complete with all unnecessary code removed or secured properly.

