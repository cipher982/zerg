# WebSocket Contract Hardening — Road-map

Single source-of-truth = **`asyncapi/chat.yml`**.
Everything else (code-gen, validation, tests, CI gates) grows out of that one
file.  The tasks below are grouped in successive, *mergeable* phases; each
brings immediate value and leaves the codebase in a green state.

----------------------------------------------------------------------
## Phase 1 — AsyncAPI spec & code-generation

### 1.1  Draft `asyncapi/chat.yml`  (✅ once merged)
* [x] Describe global info (`title`, `version`, etc.)
* [x] Channels
  * [x] `system`   – control frames (subscribe / unsubscribe / error / …)
  * [x] `thread/{thread_id}`   – history, stream_start/stream_chunk/stream_end
  * [x] `agent/{agent_id}`    – agent_event
* [x] Components / schemas for every payload type (Ping, Pong, Error,
      ThreadHistory added)
* [x] Example messages for each operation (covering ping/pong, stream,
      agent_event, etc.)

### 1.2  Validation in CI
* [x] Add `npx @asyncapi/cli validate asyncapi/chat.yml` (with legacy fallback) to **pre-commit hook**
* [x] `scripts/validate-asyncapi.sh` runs as first step of `make test` so CI
      fails on invalid spec.

### 1.3  Code-generation targets
* [x] Backend (Rust):
  * output dir `backend/zerg/ws_schema/` (temporary, not part of git)
  * command: `asyncapi-generator -o … asyncapi-rust`
* [x] Frontend (TypeScript → wasm-bindgen):
  * output dir `frontend/generated/`
  * command: `npx @asyncapi/typescript-codegen …`
* [x] Script `scripts/regen-ws-code.sh` invoked by `make regen-ws-code`
  * script now exits **gracefully** when the Rust template is not yet
    available on npm or the network is offline (prints a ⚠️  warning, but keeps
    CI green).  Remove the skip-logic once `asyncapi-rust` is published.
* [ ] Rust template `asyncapi-rust` published to npm so code-gen actually
      produces `backend/zerg/ws_schema/`  *(waiting on upstream release)*
* [x] CI job calling `make ws-code-diff-check` to enforce no drift
* [x] Makefile target `ws-code-diff-check` added (runs regen + git diff)

----------------------------------------------------------------------
## Phase 2 — Runtime payload validation

### 2.1  Backend
* [ ] Enable `schemars::JsonSchema` on generated structs *(blocked until Rust
      structs exist – see Phase 1.3 template note)*
* [x] Central validation in WS handler:
  * pydantic schema map validates every inbound payload
  * on failure sends `{type:"error", error:"INVALID_PAYLOAD"}` then closes with **1002**

### 2.2  Frontend
* [ ] Bundle JSON Schema (compiled from AsyncAPI) via `ajv8` wasm-pack feature
* [x] Lightweight shape check in ws_client_v2 (prevents obvious malformed
      frames, closes with 1002)
* [ ] Full schema validation + UI toast / badge colouring

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

