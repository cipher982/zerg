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
  * [x] `thread:{thread_id}`   – history, stream_start/stream_chunk/stream_end *(delimiter switched to `:` to match runtime topics)*
  * [x] `agent:{agent_id}`    – agent_event
* [x] Components / schemas for every payload type (Ping, Pong, Error,
      ThreadHistory **+ new SendMessage**)
* [x] Example messages for each operation (covering ping/pong, stream,
      agent_event, etc.)

### 1.2  Validation in CI
* [x] Add `npx @asyncapi/cli validate asyncapi/chat.yml` (with legacy fallback) to **pre-commit hook**
* [x] `scripts/validate-asyncapi.sh` runs as first step of `make test` so CI
      fails on invalid spec.

### 1.3  Code-generation targets
* [x] Backend (Rust):
  * output dir `backend/zerg/ws_schema/` (temporary, not part of git)
  * command: uses `asyncapi-rust-ws-template` (community-maintained, see npm/github: kanekoshoyu/asyncapi-rust-ws-template)
  * template can be overridden via `ASYNCAPI_RUST_TEMPLATE` env var for vendoring or pinning.
* [x] Frontend (TypeScript → wasm-bindgen):
  * output dir `frontend/generated/`
  * command: uses `modelina` for TypeScript model generation.
* [x] Script `scripts/regen-ws-code.sh` invoked by `make regen-ws-code`
  * script is robust to template/network issues, supports parameterization, and is documented.
* [x] Node version pinned via `.nvmrc` (Node 20).
* [x] CI job calling `make ws-code-diff-check` to enforce no drift
* [x] Makefile target `ws-code-diff-check` added (runs regen + git diff)
* [x] Lessons learned: prefer HTTPS for git/npm, pin tool versions, and document all changes.

----------------------------------------------------------------------
## Phase 2 — Runtime payload validation

### 2.1  Backend
* n/a  `schemars::JsonSchema` (backend is Python; Rust code is client SDK only)
* [x] Central validation in WS handler (schema map + 1002 close)
* [x] Heart-beat watchdog – server pings every 30 s, disconnects after 60 s without *pong* (code **4408**)

### 2.2  Frontend
* [ ] Bundle JSON Schema (compiled from AsyncAPI) via `ajv8` wasm-pack feature
* [x] Lightweight envelope check in `WsClientV2` (prevents malformed frames)
* [x] Auto-reply with `pong` when server sends `ping`
* [ ] Full JSON Schema validation + UI toast / badge colouring for 1002 / 4401 / 4408

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
### Lessons learned so far (Jul 2025)

• Matching channel addresses to the runtime `thread:{id}` / `agent:{id}` early
  prevented SDK contract drift.
• A single–sided heartbeat (server → ping, client → pong) is simpler than
  dual-timers and plays nicer with multiple open tabs.
• Keeping one central Pydantic schema map (`websocket/handlers.py`) makes it
  trivial to add validation for new message types.
• The generated Rust artefacts are for *client* SDKs; `schemars` support on
  them is unnecessary for the Python backend — documenting this avoided new
  contributor confusion.

----------------------------------------------------------------------
### Next immediate steps

1. Bundle compiled JSON Schema into the WASM build, validate incoming frames
   with `ajv8`, and surface protocol errors via toast – completes Phase 2.
2. Add `WsBadge` colour coding + toast mapping for 1002 / 4401 / 4408.
3. Capture a Pact contract from the frontend happy-path to kick-off Phase 3.

----------------------------------------------------------------------
### Nice-to-have / stretch

* [ ] Auto-publish interactive AsyncAPI docs to GitHub Pages
* [ ] Mock server for local-dev (`asyncapi mock`) – front-end can boot without backend
