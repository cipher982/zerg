"""Unit-test for Gmail *watch* initialisation on trigger creation.

The router hook should invoke ``EmailTriggerService.initialize_gmail_trigger``
which stores ``history_id`` and ``watch_expiry`` in the trigger's ``config``
JSON.  We monkey-patch the private helper so no external network call is made.
"""

from __future__ import annotations

import pytest


@pytest.mark.asyncio
async def test_gmail_trigger_initialises_watch(client, monkeypatch, db_session):
    """Creating a gmail trigger should persist watch metadata."""

    # ------------------------------------------------------------------
    # Patch stub to deterministic values so assertion is simple
    # ------------------------------------------------------------------
    from zerg.services import email_trigger_service as svc_mod  # noqa: WPS433 – local import

    def _dummy_start_watch():  # noqa: D401 – sync stub
        return {"history_id": 42, "watch_expiry": 9999999999999}

    monkeypatch.setattr(svc_mod.EmailTriggerService, "_start_gmail_watch_stub", staticmethod(_dummy_start_watch))

    # ------------------------------------------------------------------
    # Connect Gmail so a refresh_token exists on the dev user row
    # ------------------------------------------------------------------
    from zerg.routers import auth as auth_router  # patch token exchange

    monkeypatch.setattr(
        auth_router, "_exchange_google_auth_code", lambda _code: {"refresh_token": "dummy", "access_token": "ignore"}
    )

    connect_resp = client.post("/api/auth/google/gmail", json={"auth_code": "stub"})
    assert connect_resp.status_code == 200, connect_resp.text

    # ------------------------------------------------------------------
    # Create agent & gmail trigger
    # ------------------------------------------------------------------
    agent_payload = {
        "name": "Gmail Watch Agent",
        "system_instructions": "sys",
        "task_instructions": "task",
        "model": "gpt-mock",
    }

    resp = client.post("/api/agents/", json=agent_payload)
    agent_id = resp.json()["id"]

    trigger_payload = {
        "agent_id": agent_id,
        "type": "email",
        "config": {"provider": "gmail"},
    }

    trg_resp = client.post("/api/triggers/", json=trigger_payload)
    assert trg_resp.status_code == 201, trg_resp.text

    trg_json = trg_resp.json()

    # config must now include history_id & watch_expiry
    assert trg_json["config"].get("history_id") == 42
    assert trg_json["config"].get("watch_expiry") == 9999999999999
