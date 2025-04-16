import logging

from dotenv import load_dotenv
from fastapi import FastAPI
from fastapi.responses import JSONResponse
from starlette.middleware.base import BaseHTTPMiddleware
from starlette.middleware.cors import CORSMiddleware

from zerg.app.database import Base

# Import our application modules directly from their source
from zerg.app.database import engine
from zerg.app.routers.agents import router as agents_router
from zerg.app.routers.models import router as models_router
from zerg.app.routers.threads import router as threads_router
from zerg.app.routers.websocket import router as websocket_router

# from zerg.app.websocket import EventType
# from zerg.app.websocket import connected_clients

# Load environment variables
load_dotenv()

# Create DB tables if they don't exist
Base.metadata.create_all(bind=engine)

# Create the FastAPI app
app = FastAPI(
    # Make FastAPI handle routes both with and without trailing slashes
    # This ensures /api/agents and /api/agents/ are treated the same
    redirect_slashes=False
)

# Add CORS middleware with all necessary headers
app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],  # For development - allow all origins
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
    expose_headers=["*"],
)

# Add a middleware to ensure all responses include CORS headers


class EnsureCORSMiddleware(BaseHTTPMiddleware):
    async def dispatch(self, request, call_next):
        try:
            response = await call_next(request)
        except Exception as e:
            # Catch all exceptions and return a 500 with CORS headers
            logger.error(f"Error handling request: {str(e)}")
            response = JSONResponse(status_code=500, content={"detail": "Internal Server Error"})

        # Ensure CORS headers are present on all responses, even errors
        response.headers["Access-Control-Allow-Origin"] = "*"
        response.headers["Access-Control-Allow-Methods"] = "GET, POST, PUT, DELETE, OPTIONS, PATCH"
        response.headers["Access-Control-Allow-Headers"] = "*"

        return response


# Apply the middleware
app.add_middleware(EnsureCORSMiddleware)


# Add an OPTIONS handler for CORS preflight requests
@app.options("/{rest_of_path:path}")
async def options_handler(rest_of_path: str):
    """Global OPTIONS handler for CORS preflight requests"""
    logger.info(f"Handling OPTIONS request for: /{rest_of_path}")

    # Return a response with all necessary CORS headers
    response = JSONResponse(content={"message": "OK"})
    response.headers["Access-Control-Allow-Origin"] = "*"
    response.headers["Access-Control-Allow-Methods"] = "GET, POST, PUT, DELETE, OPTIONS, PATCH"
    response.headers["Access-Control-Allow-Headers"] = "Content-Type, Authorization, X-Requested-With, Accept"
    response.headers["Access-Control-Max-Age"] = "86400"  # Cache preflight for 24 hours

    return response


# Include our API routers
app.include_router(agents_router)  # Already has /api/agents prefix
app.include_router(threads_router)  # Already has /api/threads prefix
app.include_router(websocket_router)  # Already has /api prefix
app.include_router(models_router)  # Add models router for /api/models

# Set up logging
logger = logging.getLogger(__name__)


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
