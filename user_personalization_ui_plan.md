# User Personalisation & Identity UI – Implementation Plan

> **Audience:** A full-stack engineer who is **new to this code-base** and has to add “user identity” polish after Google Sign-In was introduced.  This document is intentionally self-contained – you should be able to start coding without reading any other internal docs first.

---

## 1  Why do we need this?

Authentication landed: users can log in with Google and we issue our own JWT.  **Nothing in the UI changes afterwards**, so the product still feels anonymous and multi-tenant.  Our goals:

1. Visually confirm to the user that they are signed in (avatar, name, online status).
2. Give access to a *Profile* page where users can edit basic preferences (display name, avatar, theme …).
3. Make data feel *personal* (filter lists to “My agents” by default, etc.).


## 2  TL;DR for the busy dev

• Add `display_name`, `avatar_url`, timestamps to the **User** model.  
• New endpoints: `GET /api/users/me`, `PUT /api/users/me`.  
• Front-end: load `CurrentUser` into global `AppState`, show circle avatar top-right, dropdown (`Profile • Settings • Logout`).  
• Build `/profile` page: read/write prefs.  
• Sprinkle personal touches (greeting, “My agents” filter, avatar beside chat messages).  
Estimated **3–4 dev-days** end-to-end.


## 3  Code-base Crash-Course (60 seconds)

| Area | Tech | Entry point | Quick mental model |
|------|------|------------|----------------------|
| Backend | Python 3.12 + FastAPI | `backend/zerg/main.py` | REST & WS, event bus pattern, SQLAlchemy models |
| Frontend | Rust → WebAssembly | `frontend/src/lib.rs` | Elm-style Msg/Update, global `AppState`, topic-based WebSocket |
| Auth (new) | Google ID-token → JWT | `backend/zerg/routers/auth.py` • `frontend/src/utils::current_jwt` | Already exchanges Google token and stores JWT in localStorage |


## 4  Proposed UX Additions

### 4.1 Global Top-Bar

```
┌─────────────────────────────────────────────────────────────────────────┐
│ Agent Platform                                    • • •  🟢 A          │
└─────────────────────────────────────────────────────────────────────────┘
```

• `🟢` dot = websocket connected.  
• `A` = user avatar (first letter fallback).  
• Click → dropdown: **Profile • Settings • Sign out**.

### 4.2 Profile Page (`/#/profile`)

| Avatar upload | Display name | Email (read-only) | Theme switch | Time zone |

`Save` button triggers `PUT /users/me`.

### 4.3 Dashboard tweaks
* Default filter: **“My agents”** (owned_by == current_user.id).  
* Agents list shows an **Owner** column.  
* Header text “Welcome back, {first_name}!”.

### 4.4 Chat view
* Own messages are prefixed with small avatar & display_name.


## 5  Backend Tasks – Status

| # | Task | Status | Notes |
|---|------|--------|-------|
| 1 | Extend `User` model with display_name, avatar_url, prefs, last_login | **DONE** (`models/models.py`) |
| 2 | Add `crud.update_user()` helper | **DONE** |
| 3 | Add/extend schemas `UserOut`, **new** `UserUpdate` | **DONE** |
| 4 | Create **users router** <br/>• `GET /api/users/me` <br/>• `PUT /api/users/me` | **DONE** (`routers/users.py`) |
| 5 | Emit `USER_UPDATED` over EventBus & WebSocket (`user:{id}` topic) | **DONE** – Topic manager hooked up |
| 6 | Include name / avatar in issued JWT | **BACKLOG / optional** |

### New regression tests

`backend/tests/test_users.py` covers:

• Fetching the current profile (GET) returns valid structure.  
• Updating profile (PUT) persists and reflects on subsequent GET.  
• `USER_UPDATED` EventBus message is emitted (captured with `AsyncMock`).

All backend tests now: **102 passed, 15 skipped**.


## 6  Frontend Tasks

### 6.1 Domain model  ✅ *implemented*
Rust structs are now shipped in `frontend/src/models.rs`:
```
pub struct CurrentUser { … }

// AppState (frontend/src/state.rs)
pub current_user: Option<CurrentUser>,
pub logged_in: bool,
```

They deserialize directly from `/api/users/me`.

### 6.2 Load flow  ✅ *complete*
1 After `google_auth_login()` the Rust side now:
   • stores JWT -> localStorage (existing)
   • calls **new** `ApiClient::fetch_current_user()`
   • dispatches **Message::CurrentUserLoaded**.
2 `update.rs` stores `state.current_user` & sets `logged_in=true`.
3 Subscribes to topic `user:{id}` once profile is known; WS `user_update` frames now refresh avatar live.

New helper fns in `ApiClient`:
```rust
async fn fetch_current_user() -> Result<String, JsValue>
async fn update_current_user(patch_json: &str) -> Result<String, JsValue>
```

### 6.3 Components  ✅
* **AvatarBadge** – implemented in `frontend/src/components/avatar_badge.rs`.
* **UserMenu** – implemented & mounted automatically – shows Avatar, dropdown (Profile / Logout) and updates on WS events.
* **ProfilePage** – two-column form; messages: `ProfileFieldChanged`, `SaveProfile`.

### 6.4 Routing  ⚠️ *partial*
Hash-based deep-linking (`#/profile`) is **not yet wired**.  The Profile page
opens via the Avatar dropdown (`ToggleView(Profile)` message) but changing
`location.hash` directly does nothing.  Add a small `window.onhashchange`
listener that dispatches the correct message.

### 6.5 Sprinkle personalisation  🚧 *TODO*
* Replace “Agent Platform” splash header with **Welcome, {display_name || email}**.
* Dashboard table default scope = **My agents** (owned_by == current_user.id) and show **Owner** column.
* Show tiny avatar + display_name on own chat messages.


## 7  Non-Code Work

| Task | Owner | Notes |
|------|-------|-------|
| Avatar placeholder design | Design | Use initials + random bg colour (Material 3 spec) |
| Copywriting | PM | Revise empty-state texts: “Sign in to create your first agent” |
| Accessibility review | QA | Ensure avatar menu usable with keyboard + aria-labels |


## 8  Stretch Ideas
* Token expiry toast with “Refresh session” button.  
* Usage meter (“18K tokens this month”).  
* Organisation / team switcher in avatar dropdown.  
* Dark / light mode preference persisted in `prefs` JSON.


## 9  Suggested Implementation Sequence & Time-line

| Day | Deliverable |
|-----|-------------|
| 0   | **DONE** – DB columns, CRUD helper, schemas, routes, tests |
| 1   | **DONE** – Frontend plumbing: CurrentUser in state, fetch after login |
| 1½  | **DONE** – WS `user:{id}` subscription & live updates |
| 2   | **DONE** – AvatarBadge & UserMenu in header (incl. WS status) |
| 2½  | **DONE** – Profile page UI (basic), WS live update wiring |
| 3   | IN PROGRESS – hash-router, dashboard “My agents” filter, chat avatars |
| 4   | Outstanding personalisation polish (header greeting, owner column) |
| 5   | Cross-browser test, docs/screenshots |


## 10  Testing Matrix

| Area | Test | Tool |
|------|------|------|
| API  | unauth → 401,  auth → 200 | pytest |
| Front | login → avatar visible | wasm-bindgen-tests / Playwright |
| WS    | update display_name in tab A → dropdown in tab B refreshes | Playwright (multi-page) |


## 11  Definition of “Done”

1. User sees their avatar & name after signing in.  
2. Profile page persists changes immediately and broadcasts via WS.  
3. Dashboard defaults to *My agents* and shows owner column.  **(OPEN)**
4. Wasm-bindgen test exists for `CurrentUser` deserialization.  **(OPEN)**
5. Unit + integration tests green on backend.  
6. No regressions in existing CI suites.


---

### Appendix A – Useful file paths

```
backend/zerg/models/models.py        ← SQLAlchemy models (add columns here)
backend/zerg/routers/                ← add users.py router
backend/zerg/dependencies/auth.py    ← current_user dep (update last_login)
frontend/src/state.rs                ← add current_user field
frontend/src/models.rs               ← define CurrentUser struct
frontend/src/components/             ← new AvatarBadge, UserMenu, ProfilePage
```

Good luck — and ping @product if anything is unclear!  🚀


---

## 12  Outstanding Work (May 2025 audit)

The following tasks remain before we can close the feature flag:

1. Front-end hash-based routing (`#/profile`).
2. Header greeting – replace static “AI Agent Platform” text.
3. Dashboard: default **My agents** filter + Owner column.
4. Chat bubbles: show avatar + display_name for own messages.
5. Add display_name & avatar_url to issued JWT (optional, nice-to-have).
6. Add wasm-bindgen test that deserialises the `CurrentUser` JSON payload.

Once these are merged we should re-run this document review.
