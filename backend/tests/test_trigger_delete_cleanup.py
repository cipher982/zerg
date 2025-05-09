"""Ensure DELETE /triggers/{id} cleans up Gmail push channel (stop_watch)."""

from __future__ import annotations

import pytest


@pytest.mark.asyncio
async def test_delete_gmail_trigger_calls_stop_watch(client, monkeypatch, db_session):
    """Deleting a gmail trigger should invoke gmail_api.stop_watch()."""

    # ------------------------------------------------------------------
    # Patch Google auth-code exchange so we can create a gmail-connected user
    # ------------------------------------------------------------------
    from zerg.routers import auth as auth_router  # noqa: WPS433 – local import

    monkeypatch.setattr(
        auth_router,
        "_exchange_google_auth_code",
        lambda _code: {"refresh_token": "dummy-refresh", "access_token": "ignore"},
    )

    # Connect Gmail which will store the (encrypted) refresh token on the
    # default dev@local user that tests rely on.
    resp = client.post("/api/auth/google/gmail", json={"auth_code": "stub"})
    assert resp.status_code == 200, resp.text

    # ------------------------------------------------------------------
    # Prepare agent + gmail trigger
    # ------------------------------------------------------------------
    agent_payload = {
        "name": "Cleanup Agent",
        "system_instructions": "sys",
        "task_instructions": "task",
        "model": "gpt-mock",
    }

    agent_id = client.post("/api/agents/", json=agent_payload).json()["id"]

    trigger_payload = {
        "agent_id": agent_id,
        "type": "email",
        "config": {"provider": "gmail"},
    }

    trg_resp = client.post("/api/triggers/", json=trigger_payload)
    trg_id = trg_resp.json()["id"]

    # ------------------------------------------------------------------
    # Monkey-patch gmail_api helpers so no real network call is made.
    # ------------------------------------------------------------------
    from zerg.services import gmail_api  # noqa: WPS433

    called: list[bool] = []

    def _fake_stop_watch(*, access_token: str):  # noqa: D401 – sync helper
        called.append(True)
        return True

    monkeypatch.setattr(gmail_api, "stop_watch", _fake_stop_watch)
    monkeypatch.setattr(gmail_api, "exchange_refresh_token", lambda _rt: "dummy-access")

    # ------------------------------------------------------------------
    # Call DELETE – should trigger our fake_stop_watch
    # ------------------------------------------------------------------
    del_resp = client.delete(f"/api/triggers/{trg_id}")
    assert del_resp.status_code == 204, del_resp.text

    # ensure stop_watch was invoked exactly once
    assert called == [True]

    # verify the trigger row is gone
    from zerg.crud import crud as crud_mod

    assert crud_mod.get_trigger(db_session, trg_id) is None
