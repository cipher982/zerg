import logging

from fastapi import APIRouter
from fastapi.responses import JSONResponse

from zerg.app.database import Base
from zerg.app.database import engine

router = APIRouter(
    prefix="/admin",
    tags=["admin"],
)

logger = logging.getLogger(__name__)


@router.post("/reset-database")
async def reset_database():
    """Reset the database by dropping all tables and recreating them. For development only."""
    try:
        logger.warning("Resetting database - dropping all tables")
        Base.metadata.drop_all(bind=engine)
        logger.info("Recreating database tables")
        Base.metadata.create_all(bind=engine)
        return {"message": "Database reset successfully"}
    except Exception as e:
        logger.error(f"Error resetting database: {str(e)}")
        return JSONResponse(status_code=500, content={"detail": f"Failed to reset database: {str(e)}"})
