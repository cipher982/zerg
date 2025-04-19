import logging
import os

from fastapi import APIRouter
from fastapi import HTTPException
from fastapi.responses import JSONResponse

from zerg.app.database import Base
from zerg.app.database import default_engine
from zerg.app.database import initialize_database

router = APIRouter(
    prefix="/admin",
    tags=["admin"],
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
        Base.metadata.drop_all(bind=default_engine)
        logger.info("Recreating database tables")
        initialize_database()
        return {"message": "Database reset successfully"}
    except Exception as e:
        logger.error(f"Error resetting database: {str(e)}")
        return JSONResponse(status_code=500, content={"detail": f"Failed to reset database: {str(e)}"})
