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

### Frontend Migration Plan

Below is a detailed plan for migrating the Rust/WASM frontend to use the API endpoints instead of localStorage:

1) Setup API Client and Models (IN PROGRESS)
   - Create an `ApiClient` struct in `network.rs` with methods for all API endpoints ✅
   - Add API data models in `models.rs` that match the backend schemas ✅
   - Implement function to handle API requests with proper error handling ✅

2) Create a Transitional Layer (IN PROGRESS)
   - Update `storage.rs` to add functions for saving/loading from API while preserving localStorage ✅
   - Enhance `save_state()` to also trigger API updates ✅ 
   - Modify `load_state()` to try loading from API if localStorage is empty ✅
   - This dual approach ensures a smooth transition without breaking existing functionality

3) Replace Direct localStorage Access (TODO)
   - Identify all places in `state.rs` and other modules that access localStorage directly
   - Replace these with calls to our new API client
   - Focus on maintaining the same app state structure while changing the data source

4) Implement Node-Agent Mapping
   - Create a mapping system between Node objects and API Agents
   - Ensure Node IDs reference their corresponding Agent IDs for proper syncing
   - Update state update functions to reflect changes from both sources

5) Add API-specific Error Handling
   - Add proper error handling for network failures
   - Implement retry mechanisms for important operations
   - Consider adding a connection status indicator

6) Test with Live Backend
   - Test all CRUD operations against the live backend
   - Verify that frontend state correctly reflects backend changes
   - Ensure that offline/online transitions work smoothly

7) Add Migration Utility
   - Create a one-time migration utility to transfer existing localStorage data to backend
   - Add a UI prompt to inform users about the migration

8) Phase Out localStorage
   - Once confident in the API integration, gradually phase out localStorage
   - First make it a fallback, then remove entirely
   - Update documentation to reflect the new architecture

### Current Status
- The API client has been set up with functions for all endpoints
- API data models have been created to match backend schemas
- A transitional approach has been implemented that allows loading/saving to both localStorage and API
- Currently working on testing the API integration and implementing the Node-Agent mapping

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

We've also started on phase 5 (Frontend Integration):
1. ✅ Created an API client with methods for all endpoints
2. ✅ Added API data models matching the backend schemas 
3. ✅ Implemented a transitional approach for saving/loading from both localStorage and API

## Next Steps

Our immediate next steps are:
1. Test the API endpoints with the transitional approach
2. Complete the replacement of direct localStorage accesses in the codebase
3. Implement proper Node-Agent mapping to ensure data syncing works correctly
4. Create a migration utility to transfer existing localStorage data to the database

To test the endpoints, you can use the built-in Swagger UI at http://localhost:8001/docs when the server is running, or use tools like curl or Postman.