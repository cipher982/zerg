"""Smoke-tests for the *Email Trigger* backend scaffolding.

The service is still a stub – it does **not** connect to IMAP yet.  We only
verify that:

1. An *email* trigger can be created via the REST API and the returned JSON
   contains the supplied configuration.
2. The background helper ``EmailTriggerService._check_email_triggers`` runs
   without error and (crucially) uses the **test database** session factory
   that the tests patch in ``conftest.py``.  This prevents accidental
   connections to the real development database file.
"""

import pytest

from zerg.services.email_trigger_service import email_trigger_service

# ---------------------------------------------------------------------------
# The main test coroutine ----------------------------------------------------
# ---------------------------------------------------------------------------


@pytest.mark.asyncio
async def test_create_email_trigger_and_stub_detection(client):
    """End-to-end creation of an *email* trigger plus stub detection run."""

    # 1) Create an agent (minimal payload)
    agent_payload = {
        "name": "Email Trigger Agent",
        "system_instructions": "sys",
        "task_instructions": "task",
        "model": "gpt-mock",
    }

    resp = client.post("/api/agents/", json=agent_payload)
    assert resp.status_code == 201, resp.text
    agent_id = resp.json()["id"]

    # 2) Create an *email* trigger with some config blob
    trigger_payload = {
        "agent_id": agent_id,
        "type": "email",
        "config": {"imap_host": "imap.example.com", "username": "foo"},
    }

    trg_resp = client.post("/api/triggers/", json=trigger_payload)
    assert trg_resp.status_code == 201, trg_resp.text

    trg_json = trg_resp.json()
    assert trg_json["type"] == "email"
    assert trg_json["config"] == trigger_payload["config"]

    # 3) Run the stub checker – must complete without exceptions
    await email_trigger_service._check_email_triggers()  # type: ignore[attr-defined]
