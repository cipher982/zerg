Below is a high‑level, sequential task list for migrating to a backend‑driven agent model with SQLite storage, then exposing the necessary API routes in FastAPI. You can treat each "phase" as a mini‑milestone in your migration.

────────────────────────────────
1) PLAN & PREP ✅
────────────────────────────────
• Decide your Agent schema: ✅
  – Basic columns: id, name, status, schedule (optional), instructions or config JSON, timestamps, etc.  
  – Decide on how to store conversation history (e.g., separate table "agent_messages" or JSON in a single column).  
• Pick a Python library or ORM (e.g., SQLModel, SQLAlchemy, or bare "sqlite3" module). ✅
  – SQLAlchemy was selected for its robustness and ORM capabilities
• Confirm your directory layout so the DB creation/engine code goes in an appropriate place (e.g., "backend/db.py"). ✅
  – Used "backend/app/database.py" for database setup

────────────────────────────────
2) SET UP DATABASE & MODELS (SQLALCHEMY / SQLMODEL) ✅
────────────────────────────────

(1) Create a "db.py" or "database.py" in your backend/ folder: ✅
    – Initialize a SQLite engine (e.g., "sqlite:///app.db").  
    – Initialize a session for queries.  
    – Possibly add an Alembic migration setup if you want versioned migrations.

(2) Create a "models.py" to define your Agent class for SQLAlchemy/SQLModel: ✅
    ┌─────────────────────────────────────────
    class Agent(Base):  # or SQLModel
        __tablename__ = "agents"
        
        id = Column(Integer, primary_key=True, index=True)
        name = Column(String, nullable=False)
        status = Column(String, default="idle")
        instructions = Column(Text)      # Or JSON
        schedule = Column(String)        # e.g. a CRON string
        created_at = Column(DateTime)
        updated_at = Column(DateTime)
    └─────────────────────────────────────────

(3) Create the DB tables: ✅
    – If using Alembic: "alembic revision --autogenerate && alembic upgrade head."  
    – Or run a "Base.metadata.create_all(engine)" approach if you want a simpler approach.
    – We used the simpler Base.metadata.create_all(engine) approach in main.py

────────────────────────────────
3) CREATE REPOSITORY OR CRUD UTILS ✅
────────────────────────────────
• In a file like "crud_agents.py" (or just in "main.py" for a start), define functions to handle: ✅
  – get_agents()  
  – get_agent_by_id(agent_id)  
  – create_agent(agent_data)  
  – update_agent(agent_id, updated_data)  
  – delete_agent(agent_id)
  – Additionally, we've implemented get_agent_messages() and create_agent_message()

────────────────────────────────
4) ADD FASTAPI ENDPOINTS ✅
────────────────────────────────
• In "backend/main.py" or a dedicated router (like "backend/routers/agents.py"), add REST routes: ✅

(1) GET /api/agents ✅
   – Returns a list of all agents from DB  
   – Possibly accept query params for filtering or searching

(2) POST /api/agents ✅
   – Accepts a JSON body with agent data  
   – Creates a new agent in DB  
   – Returns the created agent object

(3) GET /api/agents/{agent_id} ✅
   – Return a single agent by ID  
   – 404 if not found

(4) PUT /api/agents/{agent_id} ✅
   – Update an agent by ID  
   – Body might have name, instructions, schedule, etc.  
   – Return updated agent

(5) DELETE /api/agents/{agent_id} ✅
   – Removes agent from DB  
   – Return success or 204 no content

(6) POST /api/agents/{agent_id}/run ✅
   – For quickly triggering an agent run.  
   – Possibly queue a task or set a "status=processing."  
   – Return some immediate acknowledgment

   Additional endpoints implemented:
   - GET /api/agents/{agent_id}/messages - get messages for an agent
   - POST /api/agents/{agent_id}/messages - create a new message for an agent

────────────────────────────────
5) INTEGRATE WITH FRONTEND ⏳
────────────────────────────────
• Modify your Rust/WASM code so that instead of storing everything in localStorage, it does:
  (i) On load, fetch "GET /api/agents" to show the agent list or dashboard.  
  (ii) For creation: do a "POST /api/agents" with new agent data.  
  (iii) For edit: "PUT /api/agents/{id}"  
  (iv) For removal: "DELETE /api/agents/{id}"  
• Keep a small cache in the WASM app for quick UI, but treat the DB as the source of truth (re‑fetch from the server on refresh).  
• Potentially remove the localState "saveState()" calls or refactor them to only store ephemeral UI state (like current tab or layout).

────────────────────────────────
6) TRANSFER EXISTING HISTORY ⏳
────────────────────────────────
• If you currently store conversation history in localStorage, decide how to handle it:
  – You might create a separate table "AgentMessage" with columns: id, agent_id, role, content, timestamp. ✅
  – On user opening the canvas or agent details, fetch messages from "GET /api/agents/{id}/messages." ✅
• This ensures that the conversation or logs also live in the DB.
• Need to implement migration script to transfer existing localStorage data to the database.

────────────────────────────────
7) TEST & CLEANUP ⏳
────────────────────────────────
(currently being built in backend/tests/)
• Thoroughly test each endpoint with a REST client or automated unit tests:
  – Create an agent, fetch list, update name, delete, etc.  
• Confirm your Rust app properly updates UI states after each call.  
• Remove or comment out the old in‑browser localStorage usage to avoid confusion.  
• Expand any functionality, e.g. hooking in a scheduling library or job runner next.

────────────────────────────────
8) OPTIONAL: SECURITY & AUTH ⏳
────────────────────────────────
• If you need multi‑user or authentication:  
  – Add a user table & JWT auth in your FastAPI (or basic OAuth2).  
  – Restrict agent creation, editing, or deletion to authenticated users.  
• This can come after the basic agent CRUD is stable.

────────────────────────────────
FUTURE STEPS
────────────────────────────────
• Once your agent data is in SQLite, the next step is scheduling tasks in the background (could be APScheduler or Celery).  
• Then you can add "run logs" or "activity logs" table to store the results of each run for each agent.  
• Provide the UI real‑time updates via WebSocket or periodic "pull."  

────────────────────────────────

## Current Progress Summary

We've successfully completed the first four phases of our migration plan:
1. ✅ Planned our database schema and chose SQLAlchemy as our ORM
2. ✅ Set up the SQLite database and defined our Agent and AgentMessage models
3. ✅ Created CRUD utility functions for interacting with our models
4. ✅ Implemented API endpoints for all required agent operations

## Next Steps

Our immediate next steps are:
1. Test the API endpoints to ensure they work correctly
2. Start integrating with the frontend by updating the Rust/WASM code to use the API endpoints
3. Create a migration script to transfer existing localStorage data to the database
4. Consider implementing scheduling functionality using APScheduler

To test the endpoints, you can use the built-in Swagger UI at http://localhost:8001/docs when the server is running, or use tools like curl or Postman.