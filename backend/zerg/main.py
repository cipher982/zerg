from dotenv import load_dotenv
from fastapi import FastAPI
from fastapi.middleware.cors import CORSMiddleware
from fastapi.responses import Response

from zerg.app.database import Base

# Import our application modules directly from their source
from zerg.app.database import engine
from zerg.app.routers.agents import router as agents_router
from zerg.app.routers.openai_chat import router as openai_chat_router

# Load environment variables
load_dotenv()

# Create DB tables if they don't exist
Base.metadata.create_all(bind=engine)

app = FastAPI()

# Add CORS middleware
app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)

# Include routers
app.include_router(agents_router)  # agents_router already has prefix="/api/agents"
app.include_router(openai_chat_router)


@app.get("/")
async def read_root():
    """Root endpoint."""
    return {"message": "Welcome to the AI Agent Platform"}


@app.get("/favicon.ico")
async def get_favicon():
    """Return an empty response for favicon requests."""
    return Response(content=b"", media_type="image/x-icon")
