import logging

from dotenv import load_dotenv

# Load environment variables FIRST - before any other imports
load_dotenv()

# fmt: off
# ruff: noqa: E402
# Standard library
from pathlib import Path

# Third-party
from fastapi import FastAPI
from fastapi.middleware.cors import CORSMiddleware
from fastapi.staticfiles import StaticFiles

from zerg.constants import AGENTS_PREFIX
from zerg.constants import API_PREFIX
from zerg.constants import MODELS_PREFIX
from zerg.constants import THREADS_PREFIX
from zerg.database import initialize_database
from zerg.routers.admin import router as admin_router
from zerg.routers.agents import router as agents_router
from zerg.routers.auth import router as auth_router
from zerg.routers.email_webhooks import router as email_webhook_router
from zerg.routers.graph_layout import router as graph_router
from zerg.routers.metrics import router as metrics_router
from zerg.routers.models import router as models_router
from zerg.routers.runs import router as runs_router
from zerg.routers.system import router as system_router
from zerg.routers.threads import router as threads_router
from zerg.routers.triggers import router as triggers_router
from zerg.routers.users import router as users_router
from zerg.routers.websocket import router as websocket_router

# Email trigger polling service (stub for now)
from zerg.services.email_trigger_service import email_trigger_service
from zerg.services.scheduler_service import scheduler_service

# fmt: on

# Configure logging
logging.basicConfig(level=logging.INFO, format="%(levelname)s - %(message)s", handlers=[logging.StreamHandler()])

# Create the FastAPI app
# ---------------------------------------------------------------------------
# FastAPI application instance
# ---------------------------------------------------------------------------

# Ensure ./static directory exists before mounting.  `StaticFiles` raises at
# runtime if the path is missing, which would break unit-tests that import the
# app without running the server process.

BASE_DIR = Path(__file__).resolve().parent.parent.parent  # repo root
STATIC_DIR = BASE_DIR / "static"
AVATARS_DIR = STATIC_DIR / "avatars"

# Create folders on import so they are there in tests and dev.
AVATARS_DIR.mkdir(parents=True, exist_ok=True)

# Create FastAPI APP
app = FastAPI(redirect_slashes=True)

# Add CORS middleware with all necessary headers
app.add_middleware(
    CORSMiddleware,
    allow_origins=["http://localhost:8002"],
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
    expose_headers=["*"],
)

# Mount /static for avatars (and any future assets served by the backend)
app.mount("/static", StaticFiles(directory=str(STATIC_DIR)), name="static")

# Include our API routers with centralized prefixes
app.include_router(agents_router, prefix=f"{API_PREFIX}{AGENTS_PREFIX}")
app.include_router(threads_router, prefix=f"{API_PREFIX}{THREADS_PREFIX}")
app.include_router(models_router, prefix=f"{API_PREFIX}{MODELS_PREFIX}")
app.include_router(websocket_router, prefix=API_PREFIX)
app.include_router(admin_router, prefix=API_PREFIX)
app.include_router(email_webhook_router, prefix=f"{API_PREFIX}")
app.include_router(triggers_router, prefix=f"{API_PREFIX}")
app.include_router(runs_router, prefix=f"{API_PREFIX}")
app.include_router(auth_router, prefix=f"{API_PREFIX}")
app.include_router(users_router, prefix=f"{API_PREFIX}")
app.include_router(graph_router, prefix=f"{API_PREFIX}")
app.include_router(system_router, prefix=API_PREFIX)
app.include_router(metrics_router)  # no prefix – Prometheus expects /metrics

# Set up logging
logger = logging.getLogger(__name__)


@app.on_event("startup")
async def startup_event():
    """Initialize services on app startup."""
    try:
        # Create DB tables if they don't exist
        initialize_database()
        logger.info("Database tables initialized")

        # Start core background services ----------------------------------
        await scheduler_service.start()
        await email_trigger_service.start()

        logger.info("Background services initialised (scheduler + email triggers)")
    except Exception as e:
        logger.error(f"Error during startup: {e}")


@app.on_event("shutdown")
async def shutdown_event():
    """Clean up services on app shutdown."""
    try:
        # Stop background services (ignore errors so shutdown continues)
        await scheduler_service.stop()
        await email_trigger_service.stop()

        logger.info("Background services stopped")
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
