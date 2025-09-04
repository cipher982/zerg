from datetime import datetime
from datetime import timedelta

from zerg.crud import crud
from zerg.services.ops_service import get_summary
from zerg.services.ops_service import get_timeseries
from zerg.services.ops_service import get_top_agents


def _mk_run(
    db, agent_id: int, thread_id: int, *, started_at, finished_at=None, status="running", duration_ms=None, cost=None
):
    run = crud.create_run(db, agent_id=agent_id, thread_id=thread_id, trigger="api", status="queued")
    crud.mark_running(db, run.id, started_at=started_at)
    if finished_at is not None:
        if status == "success":
            crud.mark_finished(db, run.id, finished_at=finished_at, duration_ms=duration_ms, total_cost_usd=cost)
        elif status == "failed":
            crud.mark_failed(db, run.id, finished_at=finished_at, duration_ms=duration_ms, error="x")
        else:
            crud.mark_finished(db, run.id, finished_at=finished_at, duration_ms=duration_ms)
    return run


def test_summary_basic(db_session):
    # Create admin current user
    admin = crud.create_user(db_session, email="ops-admin@local", provider=None, role="ADMIN")

    # Agent and thread for admin
    agent = crud.create_agent(
        db_session,
        owner_id=admin.id,
        name="ops-agent",
        system_instructions="s",
        task_instructions="t",
        model="gpt-mock",
    )
    thread = crud.create_thread(db=db_session, agent_id=agent.id, title="t1", active=True, agent_state={})

    now = datetime.utcnow()
    today = now

    # 1 success with cost and duration
    _mk_run(
        db_session,
        agent.id,
        thread.id,
        started_at=today - timedelta(minutes=30),
        finished_at=today - timedelta(minutes=29),
        status="success",
        duration_ms=60000,
        cost=0.01,
    )
    # 1 failed within last hour
    _mk_run(
        db_session,
        agent.id,
        thread.id,
        started_at=today - timedelta(minutes=10),
        finished_at=today - timedelta(minutes=9),
        status="failed",
        duration_ms=10000,
    )

    # A message to count active users
    crud.create_thread_message(db=db_session, thread_id=thread.id, role="user", content="hi")

    s = get_summary(db_session, admin)
    assert s["runs_today"] >= 2
    assert s["cost_today_usd"] is None or isinstance(s["cost_today_usd"], float)
    assert "budget_user" in s and "budget_global" in s
    assert isinstance(s["active_users_24h"], int)
    assert isinstance(s["agents_total"], int)
    assert "latency_ms" in s and set(s["latency_ms"].keys()) == {"p50", "p95"}
    assert isinstance(s["errors_last_hour"], int)
    assert isinstance(s["top_agents_today"], list)


def test_timeseries_zero_fill_runs(db_session):
    admin = crud.create_user(db_session, email="ops-admin2@local", provider=None, role="ADMIN")
    agent = crud.create_agent(
        db_session, owner_id=admin.id, name="a", system_instructions="s", task_instructions="t", model="gpt-mock"
    )
    thread = crud.create_thread(db=db_session, agent_id=agent.id, title="t", active=True, agent_state={})

    base = datetime.utcnow().replace(minute=0, second=0, microsecond=0)
    h3 = base.replace(hour=3)
    h15 = base.replace(hour=15)
    _mk_run(
        db_session,
        agent.id,
        thread.id,
        started_at=h3 + timedelta(minutes=1),
        finished_at=h3 + timedelta(minutes=2),
        status="success",
    )
    _mk_run(
        db_session,
        agent.id,
        thread.id,
        started_at=h15 + timedelta(minutes=1),
        finished_at=h15 + timedelta(minutes=2),
        status="success",
    )

    series = get_timeseries(db_session, metric="runs_by_hour", window="today")
    assert len(series) == 24
    vals = {i: 0 for i in range(24)}
    for i, bucket in enumerate(series):
        vals[i] = bucket["value"]
    assert vals[3] >= 1
    assert vals[15] >= 1


def test_top_agents_ordering_and_limit(db_session):
    admin = crud.create_user(db_session, email="ops-admin3@local", provider=None, role="ADMIN")
    a1 = crud.create_agent(
        db_session, owner_id=admin.id, name="A1", system_instructions="s", task_instructions="t", model="gpt-mock"
    )
    a2 = crud.create_agent(
        db_session, owner_id=admin.id, name="A2", system_instructions="s", task_instructions="t", model="gpt-mock"
    )
    t1 = crud.create_thread(db=db_session, agent_id=a1.id, title="t1", active=True, agent_state={})
    t2 = crud.create_thread(db=db_session, agent_id=a2.id, title="t2", active=True, agent_state={})
    now = datetime.utcnow()
    # A1: 3 runs
    for _ in range(3):
        _mk_run(
            db_session,
            a1.id,
            t1.id,
            started_at=now - timedelta(minutes=5),
            finished_at=now - timedelta(minutes=4),
            status="success",
        )
    # A2: 1 run
    _mk_run(
        db_session,
        a2.id,
        t2.id,
        started_at=now - timedelta(minutes=6),
        finished_at=now - timedelta(minutes=5),
        status="success",
    )

    top = get_top_agents(db_session, window="today", limit=1)
    assert len(top) == 1
    assert top[0]["agent_id"] == a1.id
