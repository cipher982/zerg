# WebSocket Contract Hardening — Road-map

Single source-of-truth = **`asyncapi/chat.yml`**.
Everything else (code-gen, validation, tests, CI gates) grows out of that one
file.  The tasks below are grouped in successive, *mergeable* phases; each
brings immediate value and leaves the codebase in a green state.

----------------------------------------------------------------------
## Phase 1 — AsyncAPI spec & code-generation

### 1.1  Draft `asyncapi/chat.yml`  (✅ once merged)
* [ ] Describe global info (`title`, `version`, etc.)
* [ ] Channels
  * [ ] `system`   – control frames (subscribe / unsubscribe / error / …)
  * [ ] `thread/{thread_id}`   – history, stream_start/stream_chunk/stream_end
  * [ ] `agent/{agent_id}`    – agent_event
* [ ] Components / schemas for every payload type
* [ ] Example messages for each operation (used by docs & Pact later)

### 1.2  Validation in CI
* [ ] Add `npx asyncapi validate asyncapi/chat.yml` to **pre-commit hook**
* [ ] Same command runs in `make test` so CI fails on invalid spec

### 1.3  Code-generation targets
* [ ] Backend (Rust):
  * output dir `backend/zerg/ws_schema/` (temporary, not part of git)
  * command: `asyncapi-generator -o … asyncapi-rust`
* [ ] Frontend (TypeScript → wasm-bindgen):
  * output dir `frontend/generated/`
  * command: `npx @asyncapi/typescript-codegen …`
* [ ] Script `scripts/regen-ws-code.sh` invoked by `make regen-ws-code`
* [ ] CI check: `git diff --exit-code` after regen to enforce “spec updated _and_ code regenerated” rule

----------------------------------------------------------------------
## Phase 2 — Runtime payload validation

### 2.1  Backend
* [ ] Enable `schemars::JsonSchema` on generated structs
* [ ] Middleware in WS handler:
  * validate each inbound JSON against schema
  * on failure → send `{type:"error", code:"INVALID_PAYLOAD"}` and close with code 1002

### 2.2  Frontend
* [ ] Bundle JSON Schema (compiled from AsyncAPI) via `ajv8` wasm-pack feature
* [ ] Validate every incoming frame; on error:
  * toast “Protocol error – reconnecting”
  * badge turns red
  * force `ws.close()` to trigger reconnect

----------------------------------------------------------------------
## Phase 3 — Consumer-driven contract tests (Pact V4)

### 3.1  Front-end (consumer)
* [ ] Use `pact-web` to capture a happy-path interaction:
  * subscribe → expect thread_history
  * send_message → expect stream_start/chunk/end
* [ ] Commit generated `.json` pact file to `contracts/`

### 3.2  Back-end (provider)
* [ ] Add `pact_consumer` + WebSocket plugin to dev-dependencies
* [ ] Provider test spins up FastAPI app & verifies all contracts in `contracts/`

### 3.3  CI integration
* [ ] `make pact-verify` target run in backend test job

----------------------------------------------------------------------
## Phase 4 — Property-based fuzzing

* [ ] Generate proptest strategies from JSON Schema (blog pattern)
* [ ] Fuzz sequences: random valid control & user messages (max 30)
* [ ] Assert: server never panics, always responds within 100 ms

----------------------------------------------------------------------
## Phase 5 — Fail-fast UX polish

* [ ] Ping watchdog on frontend: if no frame for (`ping_interval × 2`) → force reconnect
* [ ] Error toast & WS badge colour coding for close codes 1002, 4401, 4408

----------------------------------------------------------------------
### Nice-to-have / stretch

* [ ] Auto-publish interactive AsyncAPI docs to GitHub Pages
* [ ] Mock server for local-dev (`asyncapi mock`) – front-end can boot without backend

----------------------------------------------------------------------
**Owners**

| Phase | Lead | Reviewers |
|-------|------|-----------|
| 1     | @backend-core | @frontend-core |
| 2     | @backend-core | @frontend-core |
| 3     | @qa | @backend-core, @frontend-core |
| 4     | @infra-experiments | @backend-core |
| 5     | @frontend-core | — |

----------------------------------------------------------------------
© 2025 Zerg AI