"""
LangGraph Agent Manager for the Zerg Agent Platform.

This module provides the AgentManager class that handles all LangGraph-based agent interactions.
It abstracts the underlying LLM calls and thread state management.
"""

import logging
import os
from typing import Annotated
from typing import Any
from typing import Dict
from typing import Optional
from typing import Tuple

from langchain_openai import ChatOpenAI
from langgraph.graph import END
from langgraph.graph import START
from langgraph.graph import StateGraph
from langgraph.graph.message import add_messages
from typing_extensions import TypedDict

from zerg.app.models.models import Agent as AgentModel
from zerg.app.models.models import Thread as ThreadModel

logger = logging.getLogger(__name__)


class AgentManagerState(TypedDict):
    """State definition for the LangGraph agent."""

    # Messages have the type "list". The `add_messages` function in the annotation
    # defines how this state key should be updated (appends messages instead of overwriting)
    messages: Annotated[list, add_messages]
    # Store any additional state information
    metadata: Dict[str, Any]


class AgentManager:
    """
    Manages LangGraph agent interactions and state persistence.

    This class handles:
    - Creating and managing threads
    - Building and executing LangGraph state machines
    - Persisting thread state to the database
    - Streaming responses back to clients
    """

    def __init__(self, agent_model: AgentModel):
        """Initialize the AgentManager with an agent model."""
        self.agent_model = agent_model

        # Set up the LLM
        self.llm = ChatOpenAI(
            model=agent_model.model,
            temperature=0,
            streaming=True,
            api_key=os.environ.get("OPENAI_API_KEY"),
        )

    def _build_graph(self) -> StateGraph:
        """
        Build the LangGraph state machine.

        Currently, this creates a simple chatbot, but it can be extended
        with more complex flows, tools, and conditional routing.
        """
        # Create a StateGraph with the schema defined in AgentManagerState
        graph_builder = StateGraph(AgentManagerState)

        # Add a chatbot node that will process messages
        graph_builder.add_node("chatbot", self._chatbot_node)

        # Define the flow: START -> chatbot -> END
        graph_builder.add_edge(START, "chatbot")
        graph_builder.add_edge("chatbot", END)

        # Compile the graph
        return graph_builder.compile()

    def _chatbot_node(self, state: AgentManagerState) -> Dict:
        """
        LangGraph node that processes messages with the LLM.
        This takes the current state and returns an updated state.
        """
        # Invoke the LLM with the current messages
        response = self.llm.invoke(state["messages"])

        # Return the updated state
        return {"messages": [response]}

    def get_or_create_thread(
        self, db, thread_id: Optional[int] = None, title: str = "New Thread"
    ) -> Tuple[ThreadModel, bool]:
        """
        Get an existing thread or create a new one.

        Args:
            db: Database session
            thread_id: Optional ID of an existing thread
            title: Title for a new thread if one is created

        Returns:
            Tuple of (thread, created) where created is True if a new thread was created
        """
        from zerg.app.crud import crud

        created = False

        if thread_id:
            # Try to get the existing thread
            thread = crud.get_thread(db, thread_id)
            if not thread or thread.agent_id != self.agent_model.id:
                # If not found or belongs to a different agent, create a new one
                thread = None
        else:
            # Try to find an active thread for this agent
            thread = crud.get_active_thread(db, self.agent_model.id)

        # If no thread found or specified, create a new one
        if not thread:
            thread = crud.create_thread(
                db=db,
                agent_id=self.agent_model.id,
                title=title,
                active=True,
                agent_state={},
                memory_strategy="buffer",
            )
            created = True

        return thread, created

    def add_system_message(self, db, thread: ThreadModel) -> None:
        """
        Add the system instructions as the first message in a thread.
        """
        from zerg.app.crud import crud

        # Only add if the thread is new (no messages yet)
        messages = crud.get_thread_messages(db, thread.id)
        if not messages:
            # Add the system message
            crud.create_thread_message(
                db=db,
                thread_id=thread.id,
                role="system",
                content=self.agent_model.system_instructions,
            )

    def process_message(self, db, thread: ThreadModel, content: str = None, stream=True):
        """
        Process a new user message through the LangGraph agent.

        Args:
            db: Database session
            thread: The thread to add the message to
            content: The content of the user message, None if processing existing unprocessed messages
            stream: Whether to stream the response

        Returns:
            Generator that yields response chunks if streaming,
            or the complete response if not streaming
        """
        from zerg.app.crud import crud

        # Add the user message to the database if content is provided
        if content:
            user_message = crud.create_thread_message(
                db=db,
                thread_id=thread.id,
                role="user",
                content=content,
                processed=False,  # Will be marked as processed after agent handles it
            )

        # Get unprocessed messages
        unprocessed_messages = crud.get_unprocessed_messages(db, thread.id)

        # If no unprocessed messages and no new content, nothing to do
        if not unprocessed_messages and not content:
            return

        # Build the graph
        graph = self._build_graph()

        # Prepare initial state
        # Get all previous messages from the database
        db_messages = crud.get_thread_messages(db, thread.id)
        messages = []

        # Convert database messages to the format expected by LangGraph/LangChain
        for msg in db_messages:
            if msg.tool_calls:
                # Handle tool messages
                message_dict = {
                    "role": msg.role,
                    "content": msg.content,
                }
                if msg.tool_calls:
                    message_dict["tool_calls"] = msg.tool_calls
                if msg.name:
                    message_dict["name"] = msg.name
                if msg.tool_call_id:
                    message_dict["tool_call_id"] = msg.tool_call_id
                messages.append(message_dict)
            else:
                # Handle regular messages
                messages.append({"role": msg.role, "content": msg.content})

        # Initial state
        state = {
            "messages": messages,
            "metadata": {
                "agent_id": self.agent_model.id,
                "thread_id": thread.id,
            },
        }

        # Define a function to save response chunks
        response_content = ""

        if stream:
            # Stream mode
            for event in graph.stream(state):
                if "chatbot" in event and len(event["chatbot"]["messages"]) > 0:
                    # Extract the last message added
                    last_message = event["chatbot"]["messages"][-1]

                    # If this is a chunk, yield it
                    if hasattr(last_message, "content"):
                        chunk = last_message.content
                        response_content += chunk
                        yield chunk
        else:
            # Non-streaming mode
            result = graph.invoke(state)
            if "messages" in result and len(result["messages"]) > 0:
                # Get the last message
                last_message = result["messages"][-1]
                if hasattr(last_message, "content"):
                    response_content = last_message.content
                    yield response_content

        # After streaming completes, save the full response to the database
        if response_content:
            crud.create_thread_message(
                db=db,
                thread_id=thread.id,
                role="assistant",
                content=response_content,
                processed=True,  # Assistant responses are always processed
            )

            # Mark all unprocessed messages as processed
            for msg in unprocessed_messages:
                crud.mark_message_processed(db, msg.id)

            # If we had a new message with content, mark it as processed too
            if content:
                user_message_id = user_message.id
                crud.mark_message_processed(db, user_message_id)

            # Update the thread timestamp
            crud.update_thread(db, thread.id)
