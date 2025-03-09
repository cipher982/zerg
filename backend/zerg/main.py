import json
import os

from dotenv import load_dotenv
from fastapi import FastAPI
from fastapi import HTTPException
from fastapi import WebSocket
from fastapi.middleware.cors import CORSMiddleware
from fastapi.responses import Response
from openai import OpenAI
from pydantic import BaseModel

# Import our application modules
from zerg.app.database import engine
from zerg.app.models import Base
from zerg.app.routers import agents_router

# Load environment variables
load_dotenv()

# Create DB tables if they don't exist
Base.metadata.create_all(bind=engine)

app = FastAPI()

# Add CORS middleware
app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],  # Allows all origins
    allow_credentials=True,
    allow_methods=["*"],  # Allows all methods
    allow_headers=["*"],  # Allows all headers
)

# Include routers
app.include_router(agents_router)

# Initialize OpenAI client
client = OpenAI(api_key=os.environ.get("OPENAI_API_KEY"))


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


@app.get("/")
async def read_root():
    """Root endpoint."""
    return {"message": "Welcome to the AI Agent Platform"}


@app.get("/api/models")
async def get_available_models():
    """Get available OpenAI models."""
    models = client.models.list()
    return {"models": [model.id for model in models.data]}


@app.post("/api/process-text", response_model=TextResponse)
async def process_text(request: TextRequest):
    """Process text using the model."""
    try:
        messages = []

        # Add system message if provided
        if request.system:
            messages.append({"role": "system", "content": request.system})

        # Add user message
        messages.append({"role": "user", "content": request.text})

        response = client.chat.completions.create(
            model=request.model,
            messages=messages,
        )

        return {
            "response": response.choices[0].message.content,
            "message_id": request.message_id,
        }
    except Exception as e:
        raise HTTPException(status_code=500, detail=str(e))


@app.websocket("/ws")
async def websocket_endpoint(websocket: WebSocket):
    """WebSocket endpoint for real-time communication."""
    await websocket.accept()
    try:
        while True:
            data = await websocket.receive_text()
            try:
                json_data = json.loads(data)

                messages = []

                # Add system message if provided
                if "system" in json_data and json_data["system"]:
                    messages.append({"role": "system", "content": json_data["system"]})

                # Add user message
                messages.append({"role": "user", "content": json_data["text"]})

                model = json_data.get("model", "gpt-4o")

                response = client.chat.completions.create(
                    model=model,
                    messages=messages,
                    stream=True,
                )

                # Return message_id with first chunk if provided
                if "message_id" in json_data:
                    await websocket.send_json({"type": "message_id", "message_id": json_data["message_id"]})

                # Stream the response
                for chunk in response:
                    if chunk.choices and chunk.choices[0].delta.content:
                        await websocket.send_json({"type": "chunk", "chunk": chunk.choices[0].delta.content})

                # Signal end of response
                await websocket.send_json({"type": "done"})

            except Exception as e:
                await websocket.send_json({"type": "error", "error": str(e)})
    except Exception as e:
        print(f"WebSocket error: {str(e)}")


@app.get("/favicon.ico")
async def get_favicon():
    """Return an empty response for favicon requests."""
    return Response(content=b"", media_type="image/x-icon")
