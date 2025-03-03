import os
from fastapi import FastAPI, WebSocket, HTTPException
from fastapi.middleware.cors import CORSMiddleware
from pydantic import BaseModel
from dotenv import load_dotenv
import openai
from typing import List

# Load environment variables
load_dotenv()

# Initialize OpenAI client
openai.api_key = os.getenv("OPENAI_API_KEY")
if not openai.api_key:
    raise ValueError("OPENAI_API_KEY environment variable not set")

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

class TextResponse(BaseModel):
    """Response model for text processing."""
    response: str

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
        response = openai.chat.completions.create(
            model="gpt-3.5-turbo",
            messages=[
                {"role": "system", "content": "You are a helpful assistant."},
                {"role": "user", "content": request.text}
            ],
            max_tokens=150
        )
        
        # Extract the response text
        response_text = response.choices[0].message.content
        
        # Broadcast the response to all connected WebSocket clients
        for client in connected_clients:
            try:
                await client.send_text(response_text)
            except Exception:
                # Remove clients that have disconnected
                connected_clients.remove(client)
        
        return TextResponse(response=response_text)
    except Exception as e:
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
    uvicorn.run(app, host="0.0.0.0", port=8000) 