# Zerg Platform – Google-only Auth MVP  🚧 Implementation Road-map

> **Goal:** Ship a *minimal but production-grade* authentication layer based on
> Google Sign-In (OIDC), plus a `AUTH_DISABLED` dev bypass so local hacking
> remains friction-free.  This document is a living checklist – tick boxes as
> tasks land.

-------------------------------------------------------------------------------
Legend
-------------------------------------------------------------------------------
* `[ ]` = not started   `[~]` = in progress   `[x]` = merged to `main`
* File & class hints use **monospace**.  Keep patches small and target files
  directly to simplify code-review.

-------------------------------------------------------------------------------
Stage 0 – Pre-work (scaffolding)
-------------------------------------------------------------------------------
| # | Task | Pointers | Status |
|---|------|----------|--------|
| 0.1 | Decide & store Google *Client ID* / allowed redirect URIs          | Google Cloud Console | [x] |
| 0.2 | Add secrets to `.env.example` (`GOOGLE_CLIENT_ID`, `JWT_SECRET`, `AUTH_DISABLED`) | **repo-root/.env.example** | [x] |

-------------------------------------------------------------------------------
Stage 1 – Database model & migration
-------------------------------------------------------------------------------
| # | Task | Code Location | Status |
|---|------|---------------|--------|
| 1.1 | Create **User** table (`id, email, provider, provider_user_id, is_active, timestamps`) | `backend/zerg/models/models.py` | [x] |
| 1.2 | Lightweight Alembic migration or `Base.metadata.create_all()` fallback | `backend/zerg/database.py` | [x] |
| 1.3 | CRUD helpers `get_user_by_email`, `create_user`, `get_user`            | `backend/zerg/crud/crud.py` | [x] |

-------------------------------------------------------------------------------
Stage 2 – Auth routes & token minting
-------------------------------------------------------------------------------
| # | Task | Code Location | Status |
|---|------|---------------|--------|
| 2.1 | New router **`auth.py`** with prefix `/api/auth`                     | `backend/zerg/routers/auth.py` | [x] |
| 2.2 | Endpoint **POST `/api/auth/google`** (`{ "id_token": str }`)         | same | [x] |
| 2.3 | Validate Google ID-token (`python-jose` + cached JWKS)                | same | [x] |
| 2.4 | Upsert user → issue short-lived HS256 JWT (`access_token`)            | same | [x] |
| 2.5 | Pydantic schema **`TokenOut`**                                        | `backend/zerg/schemas/schemas.py` | [x] |

-------------------------------------------------------------------------------
Stage 3 – Auth dependency & dev-mode bypass
-------------------------------------------------------------------------------
| # | Task | Code Location | Status |
|---|------|---------------|--------|
| 3.1 | Utility `get_current_user()` (`dependencies/auth.py` new file)         | `backend/zerg/dependencies/auth.py` | [x] |
| 3.2 | Implement branch: if `AUTH_DISABLED==1` → always return *dev user*     | same | [x] |
| 3.3 | If enabled: verify JWT, load user, raise 401 if missing/expired        | same | [x] |
| 3.4 | Inject `current_user: User = Depends(get_current_user)` into **ALL** existing routers (except `/models`) | each router | [x] |

> **Update (2025-05-02):** All Stage 3 items have landed.  The backend now
> enforces JWT auth across every router except the public `/models` and
> `/auth` endpoints.  Setting `AUTH_DISABLED=1` in the environment bypasses
> auth and automatically returns/creates a deterministic *dev@local* user –
> this keeps the existing test-suite and local hacking workflow functional.

-------------------------------------------------------------------------------
Stage 4 – Front-end integration
-------------------------------------------------------------------------------
| # | Task | Code Location | Status |
|---|------|---------------|--------|
| 4.1 | Add Google Identity script tag ⇒ `www/index.html`                     | `frontend/www/index.html` | [ ] |
| 4.2 | Rust/JS glue: render Google button, forward `credential` to API       | `frontend/src/components/auth.rs` (new) | [ ] |
| 4.3 | Extend **`ApiClient`** → `google_auth_login(id_token)`                | `frontend/src/network/api_client.rs` | [ ] |
| 4.4 | Persist JWT (`localStorage["zerg_jwt"]`)                              | same | [ ] |
| 4.5 | Attach `Authorization: Bearer …` in `fetch_json()` if present         | same | [ ] |
| 4.6 | WebSocket: append `?token=<jwt>` on connect                           | `frontend/src/network/ws_client_v2.rs` | [ ] |
| 4.7 | Simple *logged-in* flag in `AppState` to show / hide login modal      | `frontend/src/state.rs` | [ ] |

-------------------------------------------------------------------------------
Stage 5 – Trigger hardening (HMAC header)
-------------------------------------------------------------------------------
| # | Task | Code Location | Status |
|---|------|---------------|--------|
| 5.1 | Spec headers `X-Zerg-Timestamp`, `X-Zerg-Signature`                   | `backend/zerg/routers/triggers.py` | [ ] |
| 5.2 | Implement 5-min replay-window & HMAC-SHA256 validation                | same | [ ] |
| 5.3 | Update tests (see `tests/test_triggers.py`)                           | `backend/tests/…` | [ ] |

-------------------------------------------------------------------------------
Stage 6 – Tests & quality gates
-------------------------------------------------------------------------------
| # | Task | Code Location | Status |
|---|------|---------------|--------|
| 6.1 | Unit-tests for ID-token verification & JWT minting                    | `backend/tests/test_auth_google.py` (new) | [ ] |
| 6.2 | Integration test: unauthenticated request returns 401 (prod-mode)     | same | [ ] |
| 6.3 | Integration test: `AUTH_DISABLED=1` bypass works                      | same | [ ] |
| 6.4 | Front-end wasm-bindgen test: `google_auth_login` stores token         | `frontend/run_frontend_tests.sh` | [ ] |

-------------------------------------------------------------------------------
Stage 7 – Docs & housekeeping
-------------------------------------------------------------------------------
| # | Task | Code Location | Status |
|---|------|---------------|--------|
| 7.1 | Update **README.md** quick-start (mention Google sign-in & bypass)    | `README.md` | [ ] |
| 7.2 | Add security note to `ops_roadmap.md` → “Phase A shipped”             | `ops_roadmap.md` | [ ] |
| 7.3 | Verify `pre-commit` passes & bump `ruff.toml` exclude list if needed  | repo root | [ ] |

-------------------------------------------------------------------------------
Appendix – Dev user helper
-------------------------------------------------------------------------------
*Helper function to auto-create a deterministic user when `AUTH_DISABLED` is on.*

```python
# backend/zerg/dependencies/auth.py
DEV_EMAIL = "dev@local"

def _get_or_create_dev_user(db: Session) -> User:
    from zerg.crud import crud
    if (u := crud.get_user_by_email(db, DEV_EMAIL)):
        return u
    return crud.create_user(db, email=DEV_EMAIL, provider=None)
```

-------------------------------------------------------------------------------
This roadmap lives in **`auth_implementation_roadmap.md`** – keep it updated in
every PR so the whole team sees progress at a glance.  Happy coding! 🛠️
