"""Test automatic renewal of Gmail watch when expiry is near.

We set the trigger's ``watch_expiry`` to *yesterday* so the next call to
``EmailTriggerService._check_email_triggers`` must update the timestamp.
"""

from __future__ import annotations

import time

import pytest

from zerg.services.email_trigger_service import email_trigger_service


@pytest.mark.asyncio
async def test_gmail_watch_renewal(client, db_session, monkeypatch):
    """Expired watch should be renewed and expiry pushed into the future."""

    # Patch stub renewal for deterministic timestamp
    fixed_future_ts = int(time.time() * 1000) + 10 * 24 * 60 * 60 * 1000  # +10 days

    def _dummy_renew_stub():  # noqa: D401 – stub
        return {"history_id": 777, "watch_expiry": fixed_future_ts}

    from zerg.services.email_trigger_service import EmailTriggerService  # noqa: WPS433

    monkeypatch.setattr(
        EmailTriggerService,
        "_renew_gmail_watch_stub",
        staticmethod(_dummy_renew_stub),
    )

    # Connect Gmail so renewal can proceed
    from zerg.routers import auth as auth_router

    monkeypatch.setattr(
        auth_router, "_exchange_google_auth_code", lambda _c: {"refresh_token": "x", "access_token": "y"}
    )
    client.post("/api/auth/google/gmail", json={"auth_code": "z"})

    # Create agent and trigger with expired watch --------------------------
    agent_payload = {
        "name": "Renewal Agent",
        "system_instructions": "sys",
        "task_instructions": "task",
        "model": "gpt-mock",
    }
    agent_id = client.post("/api/agents/", json=agent_payload).json()["id"]

    expired_ts = int(time.time() * 1000) - 60 * 60 * 1000  # 1h ago

    trigger_payload = {
        "agent_id": agent_id,
        "type": "email",
        "config": {"provider": "gmail", "history_id": 5, "watch_expiry": expired_ts},
    }
    trg_json = client.post("/api/triggers/", json=trigger_payload).json()
    trg_id = trg_json["id"]

    # Invoke check loop once – should renew    ----------------------------
    await email_trigger_service._check_email_triggers()  # type: ignore[attr-defined]

    # Verify DB value updated
    from zerg.models.models import Trigger

    refreshed = db_session.query(Trigger).filter(Trigger.id == trg_id).first()
    assert refreshed is not None
    assert refreshed.config["watch_expiry"] == fixed_future_ts
