import os
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
    return sessionmaker(autocommit=False, autoflush=False, bind=engine)


def get_session_factory() -> sessionmaker:
    """Get the default session factory for the application.

    Uses DATABASE_URL from environment or falls back to default SQLite path.

    Returns:
        A sessionmaker instance
    """
    db_url = os.getenv("DATABASE_URL")
    if not db_url:
        raise ValueError("DATABASE_URL is not set")
    engine = make_engine(db_url)
    return make_sessionmaker(engine)


# Default engine and sessionmaker instances for app usage
db_url = os.getenv("DATABASE_URL")
if not db_url:
    raise ValueError("DATABASE_URL is not set")
default_engine = make_engine(db_url)
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
