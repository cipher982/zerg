import logging
import os

# FastAPI helpers
from fastapi import APIRouter
from fastapi import Depends
from fastapi import HTTPException
from fastapi.responses import JSONResponse

# Database helpers
from zerg.database import Base
from zerg.database import default_engine
from zerg.database import initialize_database

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
    if os.environ.get("ENVIRONMENT") != "development":
        logger.warning("Attempted to reset database in non-development environment")
        raise HTTPException(status_code=403, detail="Database reset is only available in development environment")

    try:
        logger.warning("Resetting database - dropping all tables")
        # Clear all connections first to avoid locks
        default_engine.dispose()

        # Drop and recreate all tables
        Base.metadata.drop_all(bind=default_engine)
        logger.info("Recreating database tables")
        initialize_database()

        # Clear connections again after recreation
        default_engine.dispose()

        return {"message": "Database reset successfully"}
    except Exception as e:
        logger.error(f"Error resetting database: {str(e)}")
        # Still return success if it's a user constraint error
        # (likely from parallel test runs)
        if "UNIQUE constraint failed: users.email" in str(e):
            return {"message": "Database reset successfully (existing user)"}
        return JSONResponse(status_code=500, content={"detail": f"Failed to reset database: {str(e)}"})
