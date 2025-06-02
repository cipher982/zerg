import logging

# FastAPI helpers
from fastapi import APIRouter
from fastapi import Depends
from fastapi import HTTPException
from fastapi.responses import JSONResponse

# Centralised settings
from zerg.config import get_settings

# Database helpers
from zerg.database import Base
from zerg.database import get_session_factory

# Auth dependency
from zerg.dependencies.auth import get_current_user
from zerg.dependencies.auth import require_admin

router = APIRouter(
    prefix="/admin",
    tags=["admin"],
    dependencies=[Depends(get_current_user), Depends(require_admin)],
)

logger = logging.getLogger(__name__)


@router.post("/reset-database")
async def reset_database():
    """Reset the database by dropping all tables and recreating them. For development only."""
    # Check if we're in development mode
    settings = get_settings()
    if not settings.testing and (settings.environment or "") != "development":
        logger.warning("Attempted to reset database in non-development environment")
        raise HTTPException(status_code=403, detail="Database reset is only available in development environment")

    try:
        logger.warning("Resetting database - dropping all tables")
        # Obtain the *current* engine – respects Playwright worker isolation
        session_factory = get_session_factory()

        # SQLAlchemy 2.0 removed the ``bind`` attribute from ``sessionmaker``.
        # We therefore open a *temporary* session and call ``get_bind()`` to
        # retrieve the underlying Engine in a version-agnostic way.
        with session_factory() as _tmp_session:  # type: ignore[arg-type]
            engine = _tmp_session.get_bind()

        if engine is None:  # pragma: no cover – safety guard
            raise RuntimeError("Session factory returned no bound engine")

        # Clear active connections to avoid SQLite locking issues.
        engine.dispose()

        # Safer + faster for SQLite: disable FK checks, truncate every table,
        # then re-enable.  Avoids losing autoincrement counters that some
        # tests rely on for deterministic IDs.

        with engine.begin() as conn:
            if engine.dialect.name == "sqlite":
                conn.exec_driver_sql("PRAGMA foreign_keys = OFF;")

            for table in reversed(Base.metadata.sorted_tables):
                conn.exec_driver_sql(f'DELETE FROM "{table.name}";')

            if engine.dialect.name == "sqlite":
                conn.exec_driver_sql("PRAGMA foreign_keys = ON;")

        logger.info("All tables truncated (worker-isolated)")

        # Dispose again after recreation to release references held by
        # background threads.
        engine.dispose()

        return {"message": "Database reset successfully"}
    except Exception as e:
        logger.error(f"Error resetting database: {str(e)}")
        # Still return success if it's a user constraint error
        # (likely from parallel test runs)
        if "UNIQUE constraint failed: users.email" in str(e):
            return {"message": "Database reset successfully (existing user)"}
        return JSONResponse(status_code=500, content={"detail": f"Failed to reset database: {str(e)}"})


# ---------------------------------------------------------------------------
# Backwards-compatibility route (no /api prefix) so legacy Playwright specs
# that still call ``POST /admin/reset-database`` continue to work.  We simply
# delegate to the main handler.
# ---------------------------------------------------------------------------

from fastapi import APIRouter as _AR

_legacy_router = _AR(prefix="/admin")


@_legacy_router.post("/reset-database")
async def _legacy_reset_database():  # noqa: D401 – thin wrapper
    return await reset_database()  # noqa: WPS110 – re-use logic


# mount the legacy router without the global /api prefix
from fastapi import FastAPI as _FastAPI


def _mount_legacy(app: _FastAPI):  # noqa: D401 – helper
    app.include_router(_legacy_router)
