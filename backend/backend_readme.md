
## Overview

This application provides a platform for creating and managing AI “agents.” An Agent encapsulates metadata such as:

• System instructions (the “role=system” message in an LLM chat)  
• Task instructions (the user’s initial instructions for the agent)  
• LLM model name (e.g., "gpt-4o")  
• Other configuration (cron schedules, JSON config, etc.)

Threads are “chat sessions” or “conversations” that belong to an Agent. Each Thread stores a sequence of messages (system, user, assistant, etc.), which can be processed by the underlying LLM. Messages can be “unprocessed” (waiting to be sent to the LLM) or “processed” (the LLM has already responded).

Higher-level tasks:

1. Create an Agent.  
2. Create a Thread for that Agent (which starts the conversation). The system message is automatically injected if it exists.  
3. Add user messages to the Thread.  
4. “Run” the thread, which processes unprocessed messages through the LLM and returns a response.  

Alternatively, you can do live, real-time interactions via WebSockets, where you subscribe to a thread, send messages, and receive broadcast updates from the server.

---

## Agents Endpoints

Agents represent the high-level “LLM persona” or configuration. The typical use cases are:

1) Create an Agent.  
2) (Optionally) retrieve or update it.  
3) (Optionally) get or create agent-level messages (this is now mostly superseded by “Threads” but remains available).

### 1) List all agents

GET /api/agents  
Returns a list of all agents in the system (paginated via query parameters skip and limit).

Example response:  
[
  {
    "id": 1,
    "name": "My Agent",
    "status": "idle",
    "system_instructions": "You are a helpful AI",
    "task_instructions": "Assist with user queries",
    "model": "gpt-4o",
    "schedule": null,
    "config": null,
    "created_at": "...",
    "updated_at": "...",
    "messages": [ ... ]  // Optional: existing agent-level messages
  },
  ...
]

### 2) Create an agent

POST /api/agents  
JSON body:  
{
  "name": "Customer Support Agent",
  "system_instructions": "You are a polite customer support chatbot",
  "task_instructions": "Help customers with their requests",
  "model": "gpt-4o",
  "schedule": "0 12 * * *",
  "config": { "some_custom_config": true }
}

On success, returns status 201 Created with the JSON of the newly-created agent.

### 3) Retrieve an agent by ID

GET /api/agents/{agent_id}

Returns a single agent object. If not found, 404 is returned.

### 4) Update an agent

PUT /api/agents/{agent_id}  
JSON body can provide any subset of updatable fields (e.g. name, system_instructions, status, etc.).

Example:  
{
  "name": "Meeting Scheduler",
  "status": "processing"
}

On success, returns 200 OK with the updated agent data. If not found, returns 404.

### 5) Delete an agent

DELETE /api/agents/{agent_id}  

Permanently deletes the agent and all associated messages and threads. Returns 204 No Content on success, or 404 if the agent_id doesn’t exist.

### 6) Get (or create) agent messages

GET /api/agents/{agent_id}/messages  
List all messages belonging to an agent. Example response:  
[
  {
    "id": 10,
    "agent_id": 1,
    "role": "system",
    "content": "You are a polite support chatbot",
    "timestamp": "..."
  },
  ...
]

POST /api/agents/{agent_id}/messages  
Creates a new message for the agent (role=user|assistant|system, content=…).  
Returns the newly created message with status 201.

(Note: Agent-level messages are an older approach. The new recommended workflow is to use Threads, described next.)

---

## Threads Endpoints

Threads represent ongoing chat sessions with a single Agent. Each Thread can store multiple messages (system, user, assistant, tool, etc.). You typically:

1) Create a Thread (which associates to an existing agent).  
2) Add user messages to that thread.  
3) Run the thread to get responses from the LLM.  

### 1) List all threads

GET /api/threads  
Query parameters: agent_id (optional), skip, limit.  
Returns an array of thread objects.

### 2) Create a thread

POST /api/threads  
JSON body (ThreadCreate schema):  
{
  "title": "My Chat Session",
  "agent_id": 1,
  "active": true,
  "memory_strategy": "buffer",
  "agent_state": { "contextKey": "contextValue" }
}

If agent_id=1 exists, creates a new thread with that agent. If `active=true`, the system will automatically deactivate any other active threads for that agent. Returns the new Thread object, with ID, timestamps, etc. If the agent doesn’t exist, returns 404.

Also:  
• Automatically adds a “system” message to the thread if the agent has system_instructions and if no messages exist yet.  

### 3) Get a thread

GET /api/threads/{thread_id}  
Returns the Thread object, or 404 if not found.

### 4) Update a thread

PUT /api/threads/{thread_id}  
JSON body (ThreadUpdate schema) can have partial fields like:  
{
  "title": "Updated Title",
  "active": false,
  "agent_state": { ... },
  "memory_strategy": "summary"
}

Returns 200 with the updated Thread. If not found, 404.

### 5) Delete a thread

DELETE /api/threads/{thread_id}  
Deletes thread and all its messages. Returns 204 on success or 404 if not found.

### 6) Get thread messages

GET /api/threads/{thread_id}/messages  
Lists messages for a given thread, in chronological order. Accepts skip and limit. Returns 404 if the thread does not exist.

### 7) Create a message in a thread

POST /api/threads/{thread_id}/messages  
JSON body (ThreadMessageCreate schema) example:  
{
  "role": "user",
  "content": "Hello, how can you help me?",
  "tool_calls": null,
  "tool_call_id": null,
  "name": null
}

This adds a new message to the thread with “processed=false” by default (meaning it hasn’t been responded to by the LLM yet). Returns 201 Created with the resulting message object.

### 8) Run a thread (invoke LLM on unprocessed messages)

POST /api/threads/{thread_id}/run  
This is the core method to actually process any unprocessed messages in a thread. It sends them to the LLM and streams back the assistant’s response. Returns a StreamingResponse with a “text/plain” content type.

• If there are no unprocessed messages, returns 200 with {"detail": "No unprocessed messages to run"}.  
• Otherwise, it performs the LLM call (via the Agent’s model, etc.), streaming partial text chunks as they arrive.  

You can consume the stream as plain text chunks or chunked lines (depending on how you parse it on the client side).

Typical usage example:  
1) POST /api/threads (create a new thread)  
2) POST /api/threads/<thread_id>/messages (add user message)  
3) POST /api/threads/<thread_id>/run (read streaming text)  

---

## WebSocket Endpoints

For real-time interaction and event broadcasting, you can use the main WebSocket endpoint at /api/ws. You can then subscribe to a thread, send messages, and automatically get broadcasts:

• Connect via ws://<domain>/api/ws.  
• Send “subscribe_thread” message with the JSON:  
  {
    "type": "subscribe_thread",
    "thread_id": 123
  }  
• Receive a “thread_history” message with past messages.  
• Send “send_message” to create new user messages:  
  {
    "type": "send_message",
    "thread_id": 123,
    "content": "Hello, WebSocket world!"
  }  
• All subscribed connections to thread 123 get a “thread_message” broadcast.  

### Typical WebSocket messages

• "ping" / "pong" – heartbeat / keepalive  
• "subscribe_thread" – subscribe to a specific thread  
• "thread_history" – the server’s response to subscription, including all past messages  
• "send_message" – client’s request to add a new user message to the thread  
• "thread_message" – server’s broadcast of new messages to everyone subscribed  

---

## Putting it all Together: Example Workflow

Below is a typical “happy path” flow:

1) Create an Agent

   POST /api/agents  
   {  
     "name": "Support Chatbot",  
     "system_instructions": "You are a helpful support representative.",  
     "task_instructions": "Help users with questions about shipping.",  
     "model": "gpt-4o"  
   }

   → Returns an Agent object with an id, e.g. 10.

2) Create a Thread for that Agent  
   POST /api/threads  
   {  
     "title": "Customer Inquiry Thread",  
     "agent_id": 10,  
     "active": true  
   }

   → Returns a Thread object with id, e.g. 42. The system message from the agent is automatically added if it exists.

3) Add a user message to the Thread  
   POST /api/threads/42/messages  
   {  
     "role": "user",  
     "content": "Hello, I want to return a product but lost my receipt."  
   }

4) Run the Thread to process unprocessed messages  
   POST /api/threads/42/run  
   → Returns a streaming text response with the LLM’s reply.  

   Confirm in the database or in subsequent GET calls that a new “assistant” message is appended with the LLM’s content.

5) (Optional) Keep sending new messages and re-run as needed.  
   • Or use the WebSocket connection at /api/ws to do the same thing in real time.

---

## Additional Notes

• You can fetch /api/models to see a small list of available model strings.  
• There is also a /api/reset-database endpoint (POST) that will drop and recreate all tables (for local development or testing). Not for production use.  
• The code includes “Agent-level messages” (older approach) and “Thread-level messages” (preferred approach). In production, you should rely on the thread approach, as it’s more robust and flexible.  
• WebSocket usage is documented in code with message types (subscribe_thread, send_message, thread_history, thread_message, etc.).  

---

## Conclusion

Through the Agents (defining how the LLM should behave) and Threads (defining conversations with that Agent), you can build a variety of chat-based AI features. Use the standard Create → Message → Run pattern for basic request/response. For real-time usage or more interactive experiences, connect via WebSocket (/api/ws). This allows sending messages and receiving updates in real time.