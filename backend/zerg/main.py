import json
import logging

from dotenv import load_dotenv
from fastapi import FastAPI
from fastapi import WebSocket
from fastapi import WebSocketDisconnect
from fastapi.responses import JSONResponse
from starlette.middleware.base import BaseHTTPMiddleware
from starlette.middleware.cors import CORSMiddleware

from zerg.app.database import Base

# Import our application modules directly from their source
from zerg.app.database import engine
from zerg.app.routers.agents import router as agents_router
from zerg.app.routers.threads import router as threads_router
from zerg.app.websocket import EventType
from zerg.app.websocket import connected_clients

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


# Include the routers
app.include_router(agents_router)
app.include_router(threads_router)

# Set up logging
logger = logging.getLogger(__name__)


# Add explicit models endpoint that the frontend is trying to access
@app.get("/api/models")
async def get_models():
    """Return available models."""
    return {"models": ["gpt-4o", "gpt-4-turbo", "gpt-3.5-turbo"]}


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


# WebSocket endpoint
@app.websocket("/ws")
async def websocket_endpoint(websocket: WebSocket):
    """Global WebSocket endpoint for system-wide events.

    This endpoint is used ONLY for broadcasting system-wide events,
    not for data operations. REST API endpoints handle all data manipulation.
    """
    await websocket.accept()
    logger.info("New client connected to global WebSocket")

    # Add client to the connected list
    connected_clients.append(websocket)

    try:
        # Send welcome message
        await websocket.send_json(
            {"type": EventType.SYSTEM_STATUS, "event": "connected", "message": "Connected to global event stream"}
        )

        # Listen for messages - primarily for ping/pong and health checks
        while True:
            data = await websocket.receive_text()
            message = json.loads(data)

            # We only support ping messages on the global socket
            if message.get("type") == "ping":
                await websocket.send_json({"type": "pong", "timestamp": message.get("timestamp", 0)})

    except WebSocketDisconnect:
        logger.info("Client disconnected from global WebSocket")
    except json.JSONDecodeError:
        logger.warning("Received invalid JSON payload")
        await websocket.send_json({"type": EventType.ERROR, "error": "Invalid JSON payload"})
    except Exception as e:
        logger.error(f"WebSocket error: {str(e)}")
        try:
            await websocket.send_json({"type": EventType.ERROR, "error": "Internal server error"})
        except Exception:
            pass
    finally:
        # Remove client from connected list
        if websocket in connected_clients:
            connected_clients.remove(websocket)
            logger.info("Removed client from connected list")
