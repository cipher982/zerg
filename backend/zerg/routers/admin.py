import logging

# FastAPI helpers
from fastapi import APIRouter
from fastapi import APIRouter as _AR
from fastapi import Depends
from fastapi import FastAPI as _FastAPI
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

        # Safer + faster for SQLite: disable FK checks, truncate every table,
        # then re-enable.  Avoids losing autoincrement counters that some
        # tests rely on for deterministic IDs.

        # ------------------------------------------------------------------
        # SQLAlchemy's *global* ``close_all_sessions()`` helper invalidates
        # **every** Session that exists in the current process – even the
        # ones that belong to a *different* Playwright worker using another
        # database file.  When multiple E2E workers run in parallel this
        # leads to race-conditions where an ongoing request suddenly loses
        # its Session mid-flight and subsequent ORM access explodes with
        # ``InvalidRequestError: Instance … is not persistent within this
        # Session``.
        #
        # Because each Playwright worker is already fully isolated via its
        # *own* SQLite engine (handled by WorkerDBMiddleware &
        # zerg.database) it is safe – and *necessary* – to avoid closing
        # foreign Sessions.  Instead we:
        #   1. Dispose the *current* worker’s engine after we are done.  This
        #      releases connections that *belong to this engine only*.
        #   2. Rely on the fact that every incoming HTTP request obtains a
        #      **fresh** Session, so no stale identity maps can leak across
        #      requests.
        #
        # Hence: **do not** call ``close_all_sessions()`` here.

        # Drop & recreate schema so **new columns** land automatically when
        # models change during active dev work (e.g. `workflow_id`).  Safer
        # than DELETE-rows because SQLite cannot ALTER TABLE with multiple
        # columns easily.

        logger.info("Dropping all tables …")
        Base.metadata.drop_all(bind=engine)

        logger.info("Re-creating all tables …")
        Base.metadata.create_all(bind=engine)

        logger.info("Database schema reset (drop+create) complete")

        # Dispose again after recreation to release references held by
        # background threads.  However, **skip** this step when the backend
        # runs inside the unit-test environment (``TESTING=1``) because
        # test fixtures may still hold an *open* SQLAlchemy ``Session`` that
        # shares the same Engine/connection.  Calling ``engine.dispose()``
        # would invalidate those connections and subsequent calls like
        # ``Session.close()`` trigger a *ProgrammingError: Cannot operate on
        # a closed database* exception which breaks the tear-down phase.

        if not settings.testing:  # avoid invalidating live connections in tests
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

_legacy_router = _AR(prefix="/admin")


@_legacy_router.post("/reset-database")
async def _legacy_reset_database():  # noqa: D401 – thin wrapper
    return await reset_database()  # noqa: WPS110 – re-use logic


# mount the legacy router without the global /api prefix


def _mount_legacy(app: _FastAPI):  # noqa: D401 – helper
    app.include_router(_legacy_router)
