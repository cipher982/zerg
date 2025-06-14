# E2E log suppression: only active when E2E_LOG_SUPPRESS=1 for test runs

from zerg.config import get_settings

_settings = get_settings()

if _settings.e2e_log_suppress:
    from zerg.e2e_logging_hacks import silence_info_logs

    silence_info_logs()

# --- TOP: Force silence for E2E or CLI if LOG_LEVEL=WARNING is set ---
import logging

# ---------------------------------------------------------------------
from dotenv import load_dotenv

# Load environment variables FIRST - before any other imports
load_dotenv()

# fmt: off
# ruff: noqa: E402
# Standard library
# fmt: on
# --------------------------------------------------------------------------
# LOGGING CONFIGURATION (dynamic, clean, less spammy):
# --------------------------------------------------------------------------
#
# - Default log level: INFO (dev-friendly)
# - Can be set at runtime with LOG_LEVEL env (e.g. LOG_LEVEL=WARNING for CI)
# - Explicitly suppresses spammy WebSocket modules to WARNING by default
#
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
from zerg.routers.mcp_servers import router as mcp_servers_router
from zerg.routers.metrics import router as metrics_router
from zerg.routers.models import router as models_router
from zerg.routers.runs import router as runs_router
from zerg.routers.system import router as system_router
from zerg.routers.threads import router as threads_router
from zerg.routers.triggers import router as triggers_router
from zerg.routers.users import router as users_router
from zerg.routers.websocket import router as websocket_router
from zerg.routers.workflow_executions import router as workflow_executions_router
from zerg.routers.workflows import router as workflows_router

# Email trigger polling service (stub for now)
# Background services ---------------------------------------------------------
#
# Long-running polling loops like *SchedulerService* and *EmailTriggerService*
# keep the asyncio event-loop alive.  When the backend is imported by *pytest*
# those tasks cause the test runner to **hang** after the last test finishes
# unless they are stopped explicitly.  To make the entire test-suite
# friction-free we skip service start-up when the environment variable
# ``TESTING`` is truthy (set automatically by `backend/tests/conftest.py`).
from zerg.services.email_trigger_service import email_trigger_service  # noqa: E402
from zerg.services.scheduler_service import scheduler_service  # noqa: E402

_log_level_name = _settings.log_level.upper()
try:
    _log_level = getattr(logging, _log_level_name)
except AttributeError:
    _log_level = logging.INFO
else:
    pass
logging.basicConfig(level=_log_level, format="%(levelname)s - %(message)s", handlers=[logging.StreamHandler()])

# Suppress verbose INFO logs from known-noisy modules (e.g., websocket connects)
for _noisy_mod in ("zerg.routers.websocket", "zerg.websocket.manager"):
    logging.getLogger(_noisy_mod).setLevel(logging.WARNING)
# --------------------------------------------------------------------------

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


@app.on_event("shutdown")
async def _shutdown_ws_manager():  # noqa: D401 – internal
    from zerg.websocket.manager import topic_manager

    await topic_manager.shutdown()


# Add CORS middleware with all necessary headers
# ------------------------------------------------------------------
# CORS – open wildcard in dev/tests, restricted in production unless env
# overrides it.  `ALLOWED_CORS_ORIGINS` can contain a comma-separated list.
# ------------------------------------------------------------------

if _settings.auth_disabled:
    cors_origins = ["*"]
else:
    cors_origins_env = _settings.allowed_cors_origins
    if cors_origins_env.strip():
        cors_origins = [o.strip() for o in cors_origins_env.split(",") if o.strip()]
    else:
        # Safe default: only allow same-origin frontend (assumes SPA served on 8002)
        cors_origins = ["https://your-domain.com", "http://localhost:8002"]

# Custom exception handler to ensure CORS headers are included in error responses
from fastapi import Request
from fastapi.responses import JSONResponse


@app.exception_handler(Exception)
async def ensure_cors_on_errors(request: Request, exc: Exception):
    """Ensure CORS headers are included even in error responses."""
    # Log the actual error for debugging
    logger.error(f"Unhandled exception: {exc}", exc_info=True)

    # Get the origin from the request
    origin = request.headers.get("origin", "*")

    # Return error response with CORS headers
    return JSONResponse(
        status_code=500,
        content={"detail": "Internal server error"},
        headers={
            "Access-Control-Allow-Origin": origin
            if origin in cors_origins or "*" in cors_origins
            else cors_origins[0]
            if cors_origins
            else "*",
            "Access-Control-Allow-Credentials": "true",
            "Access-Control-Allow-Methods": "*",
            "Access-Control-Allow-Headers": "*",
        },
    )


app.add_middleware(
    CORSMiddleware,
    allow_origins=cors_origins,
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
    expose_headers=["*"],
)

# Mount /static for avatars (and any future assets served by the backend)
app.mount("/static", StaticFiles(directory=str(STATIC_DIR)), name="static")

# ---------------------------------------------------------------------------
# Playwright worker database isolation – attach middleware early so every
# request, including those made during router setup, carries the correct
# context.
# ---------------------------------------------------------------------------

# We import lazily so local *unit-tests* that do not include the middleware
# file in their truncated import tree continue to work.
from importlib import import_module

try:
    WorkerDBMiddleware = getattr(import_module("zerg.middleware.worker_db"), "WorkerDBMiddleware")
    app.add_middleware(WorkerDBMiddleware)
except Exception:  # pragma: no cover – keep startup resilient
    # Defer logging until *logger* is available (defined right below).
    pass

# Include our API routers with centralized prefixes
app.include_router(agents_router, prefix=f"{API_PREFIX}{AGENTS_PREFIX}")
app.include_router(mcp_servers_router, prefix=f"{API_PREFIX}")  # MCP servers nested under agents
app.include_router(threads_router, prefix=f"{API_PREFIX}{THREADS_PREFIX}")
app.include_router(models_router, prefix=f"{API_PREFIX}{MODELS_PREFIX}")
app.include_router(websocket_router, prefix=API_PREFIX)
app.include_router(admin_router, prefix=API_PREFIX)
app.include_router(email_webhook_router, prefix=f"{API_PREFIX}")
app.include_router(triggers_router, prefix=f"{API_PREFIX}")
app.include_router(runs_router, prefix=f"{API_PREFIX}")
app.include_router(workflows_router, prefix=f"{API_PREFIX}")
app.include_router(workflow_executions_router, prefix=f"{API_PREFIX}")
app.include_router(auth_router, prefix=f"{API_PREFIX}")
app.include_router(users_router, prefix=f"{API_PREFIX}")
app.include_router(graph_router, prefix=f"{API_PREFIX}")
app.include_router(system_router, prefix=API_PREFIX)
app.include_router(metrics_router)  # no prefix – Prometheus expects /metrics

# ---------------------------------------------------------------------------
# Legacy admin routes without /api prefix – keep at very end so they override
# nothing and remain an optional convenience for old tests.
# ---------------------------------------------------------------------------

try:
    from zerg.routers.admin import _mount_legacy  # noqa: E402

    _mount_legacy(app)
except ImportError:  # pragma: no cover – should not happen
    pass

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
        if not _settings.testing:
            await scheduler_service.start()
            await email_trigger_service.start()

        logger.info("Background services initialised (scheduler + email triggers)")
    except Exception as e:
        logger.error(f"Error during startup: {e}")


@app.on_event("shutdown")
async def shutdown_event():
    """Clean up services on app shutdown."""
    try:
        if not _settings.testing:
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
