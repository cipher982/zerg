import logging

from dotenv import load_dotenv
from fastapi import FastAPI
from fastapi.middleware.cors import CORSMiddleware

from zerg.app.config import AGENTS_PREFIX
from zerg.app.config import API_PREFIX
from zerg.app.config import MODELS_PREFIX
from zerg.app.config import THREADS_PREFIX
from zerg.app.database import initialize_database
from zerg.app.routers.admin import router as admin_router
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
app.include_router(admin_router, prefix=API_PREFIX)

# Set up logging
logger = logging.getLogger(__name__)


@app.on_event("startup")
async def startup_event():
    """Initialize services on app startup."""
    try:
        # Create DB tables if they don't exist
        initialize_database()
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


# Redundant reset-database endpoint removed - use /admin/reset-database instead
