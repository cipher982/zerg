# WebSocket Hardening & Observability Task

*Owner: TBD*  ·  *Status: TODO*  ·  *Last updated: Jul 2025 (by OpenAI assistant)*

---

## 1 · Why this matters

The WS layer now carries **all** real-time updates (thread messages, agent
status, etc.).  In prod it works, but:  

*   unit / integration tests are brittle (difficult to assert sequences)  
*   one slow browser tab can still stall broadcasts  
*   multi-worker deployment will require an external bus  

This task hardens the design while keeping incremental-merge velocity **high**
— no “big-bang rewrite”.

---

## 2 · Current design (2025-07)

```
AgentRunner ──▶ EventBus.publish(EventType.XYZ)
                    │    (in-process async callbacks)
                    ▼
            TopicConnectionManager.broadcast(json)
                    │  (gathers all ws.send in a gather)
                    ▼
              WebSocket (per tab)
```

* **Auth**: validated _before_ `websocket.accept()`; 4401 on failure.  
* **Ping**: 30-s heartbeat, closes zombie sockets.  
* **Lock**: single `asyncio.Lock` guards `topic_subscriptions` maps.  
* **Slow client risk**: broadcast awaits each `ws.send_json` inline.  
* **Protocol**: ad-hoc JSON with keys `type`, `topic`, `data` (no version/id).  
* **Tests**: pytest uses `client.websocket_connect()` directly.

---

## 3 · Pain points / first-principles review

| Area | Finding | Consequence |
|------|---------|-------------|
| **Back-pressure** | One slow socket blocks the `asyncio.gather` fan-out. | Latency spikes, risk of memory growth. |
| **Observability** | No trace-id, no metrics, no packet ring-buffer in UI. | Hard to debug prod & CI failures. |
| **Testing DX** | Large tests need boilerplate to read frames & assert order. | Flaky WS tests, low coverage. |
| **Protocol versioning** | Schema is implicit. | Future breaking changes will be painful. |
| **Scale-out** | EventBus is in-process. | Cannot run multiple Gunicorn workers **yet**. |

---

## 4 · Proposed roadmap (incremental)

### Milestone A – Observability & DX (small PRs)

1. **Message envelope v1**  (`zerg/schemas/ws_messages.py` & FE mirror)

   ```jsonc
   {
     "v": 1,
     "id": "uuid4",   // trace-id
     "topic": "thread:123",
     "type": "thread_message_created",
     "ts": 1719958453,
     "data": { … }
   }
   ```

2. **WsHarness test helper**  (`backend/tests/ws_harness.py`)

   ```python
   async with WsHarness(client, token, topics=["thread:42"]) as ws:
       pkt = await ws.expect("thread_message_created", timeout=1)
       assert pkt["data"]["content"].endswith("Hello")
   ```

3. **UI debug overlay**  (Frontend)  
   `ws_client_v2.enable_debug_overlay()` – 200-packet ring buffer shown with
   `Ctrl+Shift+D`.

### Milestone B – Slow client isolation

* Wrap each connection in a `ClientSocket` with `asyncio.Queue(maxsize=500)`.
* Writer task drains the queue; overflow → disconnect or drop packet.
* Broadcast path switches from direct `ws.send_json()` to `client.enqueue(pkt)`.

### Milestone C – Replay / ACK (optional)

* Client sends last-seen `id` inside periodic `pong` frame.  
* REST `GET /api/ws/replay?after=<id>&topic=...` returns missed packets.

### Milestone D – Multi-worker distribution

* Replace EventBus with Redis Streams (or NATS) – same topic names.  
* Each FastAPI worker subscribes to the stream; only re-broadcasts to its
  local sockets.  
* Keep current in-memory path for **AUTH_DISABLED** dev mode (single worker).

---

## 5 · Code pointers

| Component | Path |
|-----------|------|
| WS router | `backend/zerg/routers/websocket.py` |
| Topic manager | `backend/zerg/websocket/manager.py` |
| Event bus | `backend/zerg/events/event_bus.py` |
| FE WS client | `frontend/src/network/ws_client_v2.rs` |
| FE TopicMgr | `frontend/src/network/topic_manager.rs` |

---

## 6 · Acceptance criteria (phase A)

* [ ] Envelope struct & `trace_id` emitted for every packet (BE & FE).  
* [ ] `WsHarness.expect()` used by ≥ 2 existing tests (e.g. `test_thread_message_flow`).  
* [ ] UI debug overlay toggles and shows last N packets.  
* [ ] No regression in full test-suite.

---

## 7 · Nice-to-have

* Prometheus counters: `ws_connected_total`, `ws_sent_bytes_total`.  
* Structured JSON logging including `trace_id` for each packet.  
* Mutations streamed over Server-Sent Events fallback for old browsers.

---

> _“Make it work, make it testable, then make it fast.”_

---

#EOF
