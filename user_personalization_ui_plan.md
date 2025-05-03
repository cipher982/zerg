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


## 5  Backend Tasks

1. **Extend SQL model** (`backend/zerg/models/models.py`)
   ```py
   class User(Base):
       …
       display_name = Column(String, nullable=True)
       avatar_url   = Column(String, nullable=True)
       created_at   = Column(DateTime, server_default=func.now())
       last_login   = Column(DateTime)
       prefs        = Column(JSON, default={})  # theme, timezone, …
   ```

2. **CRUD helpers** – add `update_user()`.

3. **Schemas** (`UserOut`, `UserUpdate`).

4. **Routes** (`routers/users.py` – new file)
   * `GET  /users/me` → returns `UserOut` (auth required)
   * `PUT  /users/me` → partial update via `UserUpdate`.

5. **EventBus** – fire `USER_UPDATED` so other sessions refresh.

6. (Optional) include `display_name`, `avatar_url` inside the JWT to avoid extra request on very first paint.


## 6  Frontend Tasks

### 6.1 Domain model
```
pub struct CurrentUser {
    pub id: u32,
    pub email: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub prefs: Option<serde_json::Value>,
}

AppState {
    …
    pub current_user: Option<CurrentUser>,
    pub logged_in: bool,
}
```

### 6.2 Load flow
1 After `google_auth_login()` → call `ApiClient::fetch_current_user()`.
2 Dispatch `Msg::CurrentUserLoaded(cur_user)` → set `state.current_user` & `state.logged_in=true`.
3 Subscribe to topic `user:{id}` for live updates.

### 6.3 Components
* **AvatarBadge** – renders circle image or letter fallback.
* **UserMenu** – top-bar component showing AvatarBadge, WS status dot & dropdown.
* **ProfilePage** – two-column form; messages: `ProfileFieldChanged`, `SaveProfile`.

### 6.4 Routing
Add hash-based route `#/profile` → loads ProfilePage.

### 6.5 Sprinkle personalisation
* Replace “Agent Platform” splash header with “Welcome, {display_name || email}”.
* Dashboard table default scope = current_user.id unless toggled.
* Chat bubbles include avatars.


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
| 0.5 | DB column additions, unit tests pass |
| 1   | `GET /users/me`, `PUT /users/me`, EventBus message |
| 2   | AppState.current_user, AvatarBadge & UserMenu rendered |
| 3   | Profile page, dashboard filter, chat avatars |
| 4   | Polish, cross-browser test, docs/screenshots |


## 10  Testing Matrix

| Area | Test | Tool |
|------|------|------|
| API  | unauth → 401,  auth → 200 | pytest |
| Front | login → avatar visible | wasm-bindgen-tests / Playwright |
| WS    | update display_name in tab A → dropdown in tab B refreshes | Playwright (multi-page) |


## 11  Definition of “Done”

1. User sees their avatar & name after signing in.  
2. Profile page persists changes immediately and broadcasts via WS.  
3. Dashboard defaults to *My agents* and shows owner column.  
4. Unit + integration tests green; new tests cover `users/me` endpoints.  
5. No regressions in existing CI suites.


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
