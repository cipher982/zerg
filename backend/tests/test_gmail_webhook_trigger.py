"""End-to-end smoke-test for Gmail webhook → trigger → agent execution.

The scenario:

1. Create an *agent* and an associated *email* trigger whose config marks it
   as a **gmail** provider (`{"provider": "gmail"}`).
2. Call the *connect Gmail* endpoint to store an *offline* refresh token for
   the current (dev) user.  The OAuth exchange is monkey-patched so no
   external HTTP request is made.
3. Fire the Gmail webhook callback (`/api/email/webhook/google`).  We assert
   that the SchedulerService is instructed to run the agent.
"""

from __future__ import annotations

import asyncio

import pytest

from zerg.routers import auth as auth_router
from zerg.services.scheduler_service import scheduler_service

# ---------------------------------------------------------------------------
# Helper fixtures / monkey-patching
# ---------------------------------------------------------------------------


@pytest.fixture(autouse=True)
def _patch_google_token_exchange(monkeypatch):
    """Replace the network call in `_exchange_google_auth_code` with stub."""

    fake_tokens = {"refresh_token": "test-refresh-token", "access_token": "ignore"}

    monkeypatch.setattr(auth_router, "_exchange_google_auth_code", lambda _code: fake_tokens)


# ---------------------------------------------------------------------------
# Main test coroutine
# ---------------------------------------------------------------------------


@pytest.mark.asyncio
async def test_gmail_webhook_triggers_agent(client, db_session):
    """Full flow: trigger creation → Gmail connect → webhook callback."""

    # 1) Create agent ----------------------------------------------------
    agent_payload = {
        "name": "Gmail Agent",
        "system_instructions": "sys",
        "task_instructions": "task",
        "model": "gpt-mock",
    }

    resp = client.post("/api/agents/", json=agent_payload)
    assert resp.status_code == 201, resp.text
    agent_id = resp.json()["id"]

    # 2) Create *email* trigger (provider = gmail) -----------------------
    trigger_payload = {
        "agent_id": agent_id,
        "type": "email",
        "config": {"provider": "gmail"},
    }

    trg_resp = client.post("/api/triggers/", json=trigger_payload)
    assert trg_resp.status_code == 201, trg_resp.text
    trigger_id = trg_resp.json()["id"]

    # 3) Connect Gmail (stores refresh token) ----------------------------
    connect_resp = client.post("/api/auth/google/gmail", json={"auth_code": "dummy"})
    assert connect_resp.status_code == 200, connect_resp.text

    # 4) Monkey-patch run_agent_task so we can assert call ---------------
    called = {"count": 0, "agent_id": None}

    async def _stub_run_agent_task(aid: int):  # noqa: D401 – stub
        called["count"] += 1
        called["agent_id"] = aid

    original = scheduler_service.run_agent_task
    scheduler_service.run_agent_task = _stub_run_agent_task  # type: ignore[assignment]

    try:
        # 5) Fire Gmail webhook (msg_no=1) -----------------------------
        headers = {"X-Goog-Channel-Token": "123", "X-Goog-Message-Number": "1"}
        wh_resp = client.post("/api/email/webhook/google", headers=headers)
        assert wh_resp.status_code == 202, wh_resp.text

        # Allow event-loop tasks a tiny slice
        await asyncio.sleep(0)

        assert called["count"] == 1
        assert called["agent_id"] == agent_id

        # Inspect stored last_msg_no before second callback
        from zerg.models.models import Trigger

        stored = db_session.query(Trigger).filter(Trigger.id == trigger_id).first()
        assert stored is not None
        assert stored.config.get("last_msg_no") == 1

        # 6) Fire webhook again with SAME message number → should dedup
        wh_resp2 = client.post("/api/email/webhook/google", headers=headers)
        assert wh_resp2.status_code == 202, wh_resp2.text

        await asyncio.sleep(0)
        assert called["count"] == 1, "Second webhook with same msg_no should not trigger run"
    finally:
        scheduler_service.run_agent_task = original  # type: ignore[assignment]
