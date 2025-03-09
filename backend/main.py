import json
import os
from typing import List

from dotenv import load_dotenv
from fastapi import FastAPI
from fastapi import HTTPException
from fastapi import WebSocket
from fastapi.middleware.cors import CORSMiddleware
from fastapi.responses import Response
from openai import OpenAI
from pydantic import BaseModel

# Import our application modules
from app.database import engine
from app.models import Base
from app.routers import agents_router

# Load environment variables
load_dotenv()

# Initialize OpenAI client
api_key = os.getenv("OPENAI_API_KEY")
if not api_key:
    raise ValueError("OPENAI_API_KEY environment variable not set")

client = OpenAI(api_key=api_key)

# Create DB tables
Base.metadata.create_all(bind=engine)

app = FastAPI(title="Agent Platform Backend")

# Add CORS middleware to allow requests from the frontend
app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],  # For development; restrict in production
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)

# Include our API routers
app.include_router(agents_router)


# Models
class TextRequest(BaseModel):
    """Request model for text processing."""

    text: str
    message_id: str = None  # Add message_id field
    model: str = "gpt-4o"  # Default model
    system: str = None  # Add system field


class TextResponse(BaseModel):
    """Response model for text processing."""

    response: str
    message_id: str = None  # Add message_id field


# Available AI models
AVAILABLE_MODELS = [
    {"id": "gpt-4o", "name": "GPT-4o"},
    {"id": "gpt-4o-mini", "name": "GPT-4o Mini"},
    {"id": "o3-mini", "name": "o3 Mini"},
    {"id": "gpt-4.5-preview-2025-02-27", "name": "GPT-4.5"},
]


# Connected WebSocket clients
connected_clients: List[WebSocket] = []


@app.get("/")
async def read_root():
    """Health check endpoint."""
    return {"status": "alive", "service": "agent-platform-backend"}


@app.get("/api/models")
async def get_available_models():
    """Return list of available AI models."""
    return {"models": AVAILABLE_MODELS}


@app.post("/api/process-text", response_model=TextResponse)
async def process_text(request: TextRequest):
    """Process text with OpenAI API."""
    try:
        # Get system instructions from request if available
        system_content = "You are a helpful assistant."
        if hasattr(request, "system") and request.system:
            system_content = request.system

        # Create streaming completion
        response = client.chat.completions.create(
            model=request.model,
            messages=[
                {"role": "system", "content": system_content},
                {"role": "user", "content": request.text},
            ],
            # max_tokens=300, # disabled for now, some models dont support it.
            stream=True,  # Enable streaming
        )

        accumulated_response = ""

        # Stream chunks to websocket clients
        for chunk in response:
            if chunk.choices[0].delta.content is not None:
                content = chunk.choices[0].delta.content
                accumulated_response += content

                # Prepare chunk data
                chunk_data = {
                    "type": "chunk",
                    "content": content,
                    "message_id": request.message_id,
                }

                # Broadcast chunk to all connected WebSocket clients
                for websocket_client in connected_clients:
                    try:
                        await websocket_client.send_text(json.dumps(chunk_data))
                    except Exception:
                        connected_clients.remove(websocket_client)

        # Send completion message
        completion_data = {
            "type": "completion",
            "message_id": request.message_id,
        }

        for websocket_client in connected_clients:
            try:
                await websocket_client.send_text(json.dumps(completion_data))
            except Exception:
                connected_clients.remove(websocket_client)

        return TextResponse(response=accumulated_response, message_id=request.message_id)
    except Exception as e:
        import traceback

        error_details = traceback.format_exc()
        print(f"Error processing request: {str(e)}")
        print(f"Traceback: {error_details}")
        raise HTTPException(status_code=500, detail=str(e))


@app.websocket("/ws")
async def websocket_endpoint(websocket: WebSocket):
    """WebSocket endpoint for real-time updates."""
    await websocket.accept()
    connected_clients.append(websocket)
    try:
        while True:
            # Keep the connection alive and wait for messages
            _ = await websocket.receive_text()
            # You can process incoming WebSocket messages here if needed
    except Exception:
        # Remove the client when they disconnect
        if websocket in connected_clients:
            connected_clients.remove(websocket)


@app.get("/favicon.ico")
async def get_favicon():
    """Return a simple favicon."""
    return Response(status_code=204)  # No content response, browser will use default favicon


if __name__ == "__main__":
    import uvicorn

    uvicorn.run(app, host="0.0.0.0", port=8001)
