import logging

from dotenv import load_dotenv
from fastapi import FastAPI
from fastapi.middleware.cors import CORSMiddleware
from fastapi.responses import JSONResponse

from zerg.app.config import AGENTS_PREFIX
from zerg.app.config import API_PREFIX
from zerg.app.config import MODELS_PREFIX
from zerg.app.config import THREADS_PREFIX
from zerg.app.database import Base
from zerg.app.database import engine
from zerg.app.routers.agents import router as agents_router
from zerg.app.routers.models import router as models_router
from zerg.app.routers.threads import router as threads_router
from zerg.app.routers.websocket import router as websocket_router
from zerg.app.services.scheduler_service import scheduler_service

# Load environment variables
load_dotenv()

# Create the FastAPI app
app = FastAPI(redirect_slashes=False)

# Add CORS middleware with all necessary headers
app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
    expose_headers=["*"],
)

# Include our API routers with centralized prefixes
app.include_router(agents_router, prefix=f"{API_PREFIX}{AGENTS_PREFIX}")
app.include_router(threads_router, prefix=f"{API_PREFIX}{THREADS_PREFIX}")
app.include_router(models_router, prefix=f"{API_PREFIX}{MODELS_PREFIX}")
app.include_router(websocket_router, prefix=API_PREFIX)

# Set up logging
logger = logging.getLogger(__name__)


@app.on_event("startup")
async def startup_event():
    """Initialize services on app startup."""
    try:
        # Create DB tables if they don't exist
        # Moved from global scope to prevent eager binding at import time
        Base.metadata.create_all(bind=engine)
        logger.info("Database tables initialized")

        # Start scheduler service
        await scheduler_service.start()
        logger.info("Scheduler service initialized")
    except Exception as e:
        logger.error(f"Error during startup: {e}")


@app.on_event("shutdown")
async def shutdown_event():
    """Clean up services on app shutdown."""
    try:
        await scheduler_service.stop()
        logger.info("Scheduler service stopped")
    except Exception as e:
        logger.error(f"Error stopping scheduler service: {e}")


# Root endpoint
@app.get("/")
async def read_root():
    """Return a simple message to indicate the API is working."""
    return {"message": "Agent Platform API is running"}


# Favicon endpoint is no longer needed since we use static file in the frontend
# Browsers will go directly to the frontend server for favicon.ico


# Database reset endpoint for development
@app.post("/api/reset-database")
async def reset_database():
    """Reset the database by dropping all tables and recreating them.
    This is for development purposes only and should be secured in production.
    """
    try:
        logger.warning("Resetting database - dropping all tables")
        # Drop all tables
        Base.metadata.drop_all(bind=engine)

        # Recreate all tables
        logger.info("Recreating database tables")
        Base.metadata.create_all(bind=engine)

        return {"message": "Database reset successfully"}
    except Exception as e:
        logger.error(f"Error resetting database: {str(e)}")
        return JSONResponse(status_code=500, content={"detail": f"Failed to reset database: {str(e)}"})
