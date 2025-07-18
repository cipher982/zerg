"""
Node Output Envelope Schema

Defines the standardized output structure for all workflow node executors.
Replaces inconsistent output formats with a clean value/meta separation.
"""

from datetime import datetime
from typing import Any
from typing import Dict
from typing import Literal
from typing import Optional
from typing import Union

from pydantic import BaseModel
from pydantic import Field


class NodeMetadata(BaseModel):
    """Standard metadata for all node executions."""

    # Required fields for all nodes
    node_type: Literal["tool", "agent", "trigger", "conditional"] = Field(description="Type of node that was executed")
    status: Literal["completed", "failed", "running"] = Field(description="Execution status")
    execution_time_ms: Optional[int] = Field(None, description="Execution time in milliseconds")
    started_at: Optional[datetime] = Field(None, description="When execution started")
    finished_at: Optional[datetime] = Field(None, description="When execution finished")

    # Optional standard fields
    error: Optional[str] = Field(None, description="Error message if status=failed")
    error_type: Optional[str] = Field(None, description="Type of error (e.g., ValidationError, TimeoutError)")
    retry_count: Optional[int] = Field(0, description="Number of retry attempts")
    memory_usage_mb: Optional[float] = Field(None, description="Memory usage in megabytes")
    cpu_time_ms: Optional[int] = Field(None, description="CPU time in milliseconds")


class ToolNodeMetadata(NodeMetadata):
    """Metadata specific to tool node executions."""

    node_type: Literal["tool"] = "tool"

    # Tool-specific metadata
    tool_name: Optional[str] = Field(None, description="Name of the tool that was executed")
    tool_version: Optional[str] = Field(None, description="Version of the tool")
    parameters: Optional[Dict[str, Any]] = Field(None, description="Parameters passed to the tool")
    tool_execution_time_ms: Optional[int] = Field(None, description="Time spent in tool execution (excluding overhead)")


class AgentNodeMetadata(NodeMetadata):
    """Metadata specific to agent node executions."""

    node_type: Literal["agent"] = "agent"

    # Agent-specific metadata
    agent_id: Optional[int] = Field(None, description="ID of the agent that was executed")
    agent_name: Optional[str] = Field(None, description="Name of the agent")
    model_used: Optional[str] = Field(None, description="LLM model used (e.g., gpt-4, claude-3)")
    total_tokens: Optional[int] = Field(None, description="Total tokens used in LLM calls")
    total_cost_usd: Optional[float] = Field(None, description="Total cost in USD")
    thread_id: Optional[int] = Field(None, description="Thread ID for agent conversation")


class ConditionalNodeMetadata(NodeMetadata):
    """Metadata specific to conditional node executions."""

    node_type: Literal["conditional"] = "conditional"

    # Conditional-specific metadata
    condition: Optional[str] = Field(None, description="Original condition expression")
    resolved_condition: Optional[str] = Field(None, description="Condition after variable resolution")
    evaluation_method: Optional[str] = Field(None, description="Method used for evaluation (e.g., ast_safe)")


class TriggerNodeMetadata(NodeMetadata):
    """Metadata specific to trigger node executions."""

    node_type: Literal["trigger"] = "trigger"

    # Trigger-specific metadata
    trigger_type: Optional[str] = Field(None, description="Type of trigger (manual, scheduled, webhook, etc.)")
    trigger_config: Optional[Dict[str, Any]] = Field(None, description="Configuration for the trigger")


class NodeOutputEnvelope(BaseModel):
    """
    Standardized output envelope for all node executors.

    Provides clean separation between primary result value and execution metadata.
    All node executors should return data in this format.
    """

    value: Any = Field(description="Primary result value from node execution")
    meta: Union[ToolNodeMetadata, AgentNodeMetadata, ConditionalNodeMetadata, TriggerNodeMetadata, NodeMetadata] = (
        Field(description="Execution metadata and context")
    )

    class Config:
        """Pydantic configuration."""

        json_encoders = {datetime: lambda v: v.isoformat() if v else None}


# Convenience functions for creating envelopes


def create_tool_envelope(
    value: Any,
    *,
    status: Literal["completed", "failed", "running"] = "completed",
    tool_name: Optional[str] = None,
    parameters: Optional[Dict[str, Any]] = None,
    execution_time_ms: Optional[int] = None,
    error: Optional[str] = None,
    **kwargs,
) -> NodeOutputEnvelope:
    """
    Create a standardized tool node output envelope.

    Args:
        value: Primary result from tool execution
        status: Execution status
        tool_name: Name of the tool
        parameters: Parameters passed to tool
        execution_time_ms: Execution time in milliseconds
        error: Error message if failed
        **kwargs: Additional metadata fields

    Returns:
        NodeOutputEnvelope with tool metadata
    """
    metadata = ToolNodeMetadata(
        status=status,
        tool_name=tool_name,
        parameters=parameters,
        execution_time_ms=execution_time_ms,
        error=error,
        **kwargs,
    )

    return NodeOutputEnvelope(value=value, meta=metadata)


def create_agent_envelope(
    value: Any,
    *,
    status: Literal["completed", "failed", "running"] = "completed",
    agent_id: Optional[int] = None,
    agent_name: Optional[str] = None,
    thread_id: Optional[int] = None,
    execution_time_ms: Optional[int] = None,
    error: Optional[str] = None,
    **kwargs,
) -> NodeOutputEnvelope:
    """
    Create a standardized agent node output envelope.

    Args:
        value: Primary result from agent execution
        status: Execution status
        agent_id: ID of the agent
        agent_name: Name of the agent
        thread_id: Thread ID for conversation
        execution_time_ms: Execution time in milliseconds
        error: Error message if failed
        **kwargs: Additional metadata fields

    Returns:
        NodeOutputEnvelope with agent metadata
    """
    metadata = AgentNodeMetadata(
        status=status,
        agent_id=agent_id,
        agent_name=agent_name,
        thread_id=thread_id,
        execution_time_ms=execution_time_ms,
        error=error,
        **kwargs,
    )

    return NodeOutputEnvelope(value=value, meta=metadata)


def create_conditional_envelope(
    value: Any,
    *,
    status: Literal["completed", "failed", "running"] = "completed",
    condition: Optional[str] = None,
    resolved_condition: Optional[str] = None,
    execution_time_ms: Optional[int] = None,
    error: Optional[str] = None,
    **kwargs,
) -> NodeOutputEnvelope:
    """
    Create a standardized conditional node output envelope.

    Args:
        value: Primary result from conditional evaluation
        status: Execution status
        condition: Original condition expression
        resolved_condition: Condition after variable resolution
        execution_time_ms: Execution time in milliseconds
        error: Error message if failed
        **kwargs: Additional metadata fields

    Returns:
        NodeOutputEnvelope with conditional metadata
    """
    metadata = ConditionalNodeMetadata(
        status=status,
        condition=condition,
        resolved_condition=resolved_condition,
        execution_time_ms=execution_time_ms,
        error=error,
        **kwargs,
    )

    return NodeOutputEnvelope(value=value, meta=metadata)


def create_trigger_envelope(
    value: Any,
    *,
    status: Literal["completed", "failed", "running"] = "completed",
    trigger_type: Optional[str] = None,
    trigger_config: Optional[Dict[str, Any]] = None,
    execution_time_ms: Optional[int] = None,
    error: Optional[str] = None,
    **kwargs,
) -> NodeOutputEnvelope:
    """
    Create a standardized trigger node output envelope.

    Args:
        value: Primary result from trigger execution
        status: Execution status
        trigger_type: Type of trigger
        trigger_config: Trigger configuration
        execution_time_ms: Execution time in milliseconds
        error: Error message if failed
        **kwargs: Additional metadata fields

    Returns:
        NodeOutputEnvelope with trigger metadata
    """
    metadata = TriggerNodeMetadata(
        status=status,
        trigger_type=trigger_type,
        trigger_config=trigger_config,
        execution_time_ms=execution_time_ms,
        error=error,
        **kwargs,
    )

    return NodeOutputEnvelope(value=value, meta=metadata)


# Utility functions


def is_envelope_format(data: Any) -> bool:
    """
    Check if data is in the new envelope format.

    Args:
        data: Data to check

    Returns:
        True if data is a valid envelope format
    """
    return (
        isinstance(data, dict)
        and "value" in data
        and "meta" in data
        and isinstance(data["meta"], dict)
        and "node_type" in data["meta"]
        and "status" in data["meta"]
    )


def extract_value(data: Any) -> Any:
    """
    Extract the primary value from node output.

    Envelope format only.

    Args:
        data: Node output data (envelope format)

    Returns:
        Primary value from the output
    """
    if is_envelope_format(data):
        return data["value"]
    else:
        raise ValueError("Expected envelope format node output")


def extract_metadata(data: Any) -> Optional[Dict[str, Any]]:
    """
    Extract metadata from node output.

    Args:
        data: Node output data

    Returns:
        Metadata dict if available, None otherwise
    """
    if is_envelope_format(data):
        return data["meta"]
    else:
        return None
