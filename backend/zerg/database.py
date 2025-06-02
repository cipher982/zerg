from __future__ import annotations

# Standard library
import threading
from pathlib import Path
from typing import Any, Dict
# Thread-safe caches for per-worker engines/sessionmakers --------------------

_WORKER_ENGINES: Dict[str, Engine] = {}
_WORKER_SESSIONMAKERS: Dict[str, sessionmaker] = {}
_WORKER_LOCK = threading.Lock()

# ---------------------------------------------------------------------------
# Playwright worker-based DB isolation (E2E tests)
# ---------------------------------------------------------------------------

# We *dynamically* route each HTTP/WebSocket request to its own SQLite file
# during Playwright runs.  The current worker id is injected by the middleware
# and stored in a context variable.  Importing here avoids a circular
# dependency (middleware imports *this* module).  The conditional import keeps
# the overhead negligible for production usage.

try:
    from zerg.middleware.worker_db import current_worker_id  # type: ignore

except ModuleNotFoundError:  # pragma: no cover – unit-tests without middleware
    import contextvars

    current_worker_id = contextvars.ContextVar("current_worker_id", default=None)

# from pathlib import Path  # duplicate removed above
# import threading  # duplicate
from typing import Iterator

import dotenv
from sqlalchemy import Engine
from sqlalchemy import create_engine
from sqlalchemy.orm import Session
from sqlalchemy.orm import declarative_base
from sqlalchemy.orm import sessionmaker

from zerg.config import get_settings

_settings = get_settings()

dotenv.load_dotenv()


# Create Base class
Base = declarative_base()


def make_engine(db_url: str, **kwargs) -> Engine:
    """Create a SQLAlchemy engine with the given URL and options.

    Args:
        db_url: Database connection URL
        **kwargs: Additional arguments for create_engine

    Returns:
        A SQLAlchemy Engine instance
    """
    connect_args = kwargs.pop("connect_args", {})
    if "sqlite" in db_url and "check_same_thread" not in connect_args:
        connect_args["check_same_thread"] = False

    return create_engine(db_url, connect_args=connect_args, **kwargs)


def make_sessionmaker(engine: Engine) -> sessionmaker:
    """Create a sessionmaker bound to the given engine.

    Args:
        engine: SQLAlchemy Engine instance

    Returns:
        A sessionmaker class
    """
    # `expire_on_commit=False` keeps attributes accessible after a commit,
    # preventing DetachedInstanceError in situations where objects outlive the
    # session lifecycle (e.g. during test helpers that commit and then access
    # attributes after other background DB activity).
    return sessionmaker(
        autocommit=False,
        autoflush=False,
        expire_on_commit=False,
        bind=engine,
    )


def get_session_factory() -> sessionmaker:
    """Get the default session factory for the application.

    Uses DATABASE_URL from environment or falls back to default SQLite path.

    Returns:
        A sessionmaker instance
    """
    # ---------------------------------------------------------------------
    # Tests often import the *zerg.database* module **before** they get the
    # chance to patch environment variables or monkey-patch the session
    # factory.  Raising at import time therefore breaks the entire test
    # discovery phase.  Instead of hard-failing when the variable is missing
    # we fall back to a local on-disk SQLite file.  When the test-runner sets
    # ``TESTING=1`` (done in *backend/tests/conftest.py*) we create an
    # *in-memory* SQLite engine because the file system location is
    # irrelevant and the tests patch the session/engine anyway.
    # ---------------------------------------------------------------------
    # ------------------------------------------------------------------
    # Playwright E2E tests: isolate database per worker ------------------
    # ------------------------------------------------------------------
    # When the *WorkerDBMiddleware* sets `current_worker_id` we look up a
    # dedicated engine / sessionmaker pair from an in-memory cache.  The very
    # first request for a worker lazily creates `sqlite:///./test_worker_<id>.db`
    # and initialises the schema.
    #
    # Outside the Playwright context – i.e. when *current_worker_id* is None –
    # we fall back to the original single-engine behaviour so that unit
    # tests, dev server sessions, and production deployments remain
    # unaffected.
    # ------------------------------------------------------------------

    worker_id = current_worker_id.get()

    if worker_id is None:
        # --- Legacy/shared behaviour -----------------------------------
        db_url = _settings.database_url

        if not db_url:
            if _settings.testing:
                db_url = "sqlite:///:memory:"
            else:
                db_url = "sqlite:///./app.db"

        engine = make_engine(db_url)
        return make_sessionmaker(engine)

    # --- Per-worker engine/session --------------------------------------
    if worker_id in _WORKER_SESSIONMAKERS:
        return _WORKER_SESSIONMAKERS[worker_id]

    # Lazily build the engine (thread-safe)
    with _WORKER_LOCK:
        if worker_id in _WORKER_SESSIONMAKERS:
            return _WORKER_SESSIONMAKERS[worker_id]

        # Place the database file **inside the backend directory** so the
        # helper script `backend/cleanup_test_dbs.py` can delete it after the
        # Playwright run.
        db_path = Path(__file__).resolve().parents[1] / f"test_worker_{worker_id}.db"
        db_url = f"sqlite:///{db_path}"

        engine = make_engine(db_url)
        # Create tables on first use
        initialize_database(engine)

        session_factory = make_sessionmaker(engine)

        _WORKER_ENGINES[worker_id] = engine
        _WORKER_SESSIONMAKERS[worker_id] = session_factory

        return session_factory


# Default engine and sessionmaker instances for app usage
# Reuse the same relaxed resolution logic from *get_session_factory* so that
# importing this module never crashes – the returned engine is still safe for
# overwriting in tests via ``zerg.database.default_engine = …``.

_resolved_db_url = _settings.database_url or ("sqlite:///:memory:" if _settings.testing else "sqlite:///./app.db")

default_engine = make_engine(_resolved_db_url)
default_session_factory = make_sessionmaker(default_engine)


def get_db(session_factory: Any = None) -> Iterator[Session]:
    """Dependency provider for database sessions.

    Args:
        session_factory: Optional custom session factory

    Yields:
        SQLAlchemy Session object
    """
    factory = session_factory or get_session_factory()
    db = factory()
    try:
        yield db
    finally:
        db.close()


def initialize_database(engine: Engine = None) -> None:
    """Initialize database tables using the given engine.

    If no engine is provided, uses the default engine.

    Args:
        engine: Optional engine to use, defaults to default_engine
    """
    target_engine = engine or default_engine
    Base.metadata.create_all(bind=target_engine)
