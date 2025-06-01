from __future__ import annotations

from zerg.config import get_settings

_settings = get_settings()
from typing import Any
from typing import Iterator

import dotenv
from sqlalchemy import Engine
from sqlalchemy import create_engine
from sqlalchemy.orm import Session
from sqlalchemy.orm import declarative_base
from sqlalchemy.orm import sessionmaker

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
    # Honour the *centralised* settings object.  The fallbacks mirror the
    # previous logic so behaviour remains unchanged.

    db_url = _settings.database_url

    if not db_url:
        if _settings.testing:
            db_url = "sqlite:///:memory:"
        else:
            db_url = "sqlite:///./app.db"
    engine = make_engine(db_url)
    return make_sessionmaker(engine)


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
    factory = session_factory or default_session_factory
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
