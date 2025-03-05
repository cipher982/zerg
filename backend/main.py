import os
import json
from fastapi import FastAPI, WebSocket, HTTPException
from fastapi.middleware.cors import CORSMiddleware
from pydantic import BaseModel
from dotenv import load_dotenv
from openai import OpenAI
from typing import List, Dict, Any

# Load environment variables
load_dotenv()

# Initialize OpenAI client
api_key = os.getenv("OPENAI_API_KEY")
if not api_key:
    raise ValueError("OPENAI_API_KEY environment variable not set")

client = OpenAI(api_key=api_key)

app = FastAPI(title="Agent Platform Backend")

# Add CORS middleware to allow requests from the frontend
app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],  # For development; restrict in production
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)


# Models
class TextRequest(BaseModel):
    """Request model for text processing."""

    text: str
    message_id: str = None  # Add message_id field


class TextResponse(BaseModel):
    """Response model for text processing."""

    response: str
    message_id: str = None  # Add message_id field


# Connected WebSocket clients
connected_clients: List[WebSocket] = []


@app.get("/")
async def read_root():
    """Health check endpoint."""
    return {"status": "alive", "service": "agent-platform-backend"}


@app.post("/api/process-text", response_model=TextResponse)
async def process_text(request: TextRequest):
    """Process text with OpenAI API."""
    try:
        response = client.chat.completions.create(
            model="gpt-4o",  # DONT CHANGE THIS. ITS A NEW MODEL
            messages=[
                {"role": "system", "content": "You are a helpful assistant."},
                {"role": "user", "content": request.text},
            ],
            max_tokens=300,
        )

        # Extract the response text
        response_text = response.choices[0].message.content

        print(f"Got response: {response_text}")

        # Prepare response with message_id
        response_data = {
            "response": response_text,
            "message_id": request.message_id  # Include the message_id in the response
        }

        # Broadcast the response to all connected WebSocket clients
        for websocket_client in connected_clients:
            try:
                await websocket_client.send_text(json.dumps(response_data))
            except Exception:
                # Remove clients that have disconnected
                connected_clients.remove(websocket_client)

        return TextResponse(response=response_text, message_id=request.message_id)
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
            data = await websocket.receive_text()
            # You can process incoming WebSocket messages here if needed
    except Exception:
        # Remove the client when they disconnect
        if websocket in connected_clients:
            connected_clients.remove(websocket)


if __name__ == "__main__":
    import uvicorn

    uvicorn.run(app, host="0.0.0.0", port=8001)
