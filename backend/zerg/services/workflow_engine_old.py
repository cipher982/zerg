"""
Very first, *linear* implementation of the **Workflow Execution Engine**.

This service runs a workflow **synchronously** and records its progress in the
database so that:

1.  `/api/workflow-executions/{id}/status` can already return useful data to
    the front-end.
2.  Future iterations can focus on real node execution (tools, triggers,
    agents) without worrying about the persistence boiler-plate again.

At the moment the engine:

• Creates a `WorkflowExecution` row with status _running_.
• Iterates over all nodes found in `workflow.canvas_data["nodes"]` **in order**
  – no branching / concurrency yet.
• Creates a `NodeExecutionState` row for each node and marks it _running →
  success_ (or _failed_ on exception).
• Writes a very small log string so devs can inspect the result.

If anything raises, execution status becomes _failed_ and the error is stored
both on the `WorkflowExecution` row **and** the affected
`NodeExecutionState` row so that the UI can highlight the failing node.
"""

from __future__ import annotations

import asyncio
import logging
import time
from datetime import datetime
from datetime import timezone
from typing import Any
from typing import Dict

from sqlalchemy.orm import Session

from zerg.crud import crud
from zerg.database import get_session_factory
from zerg.events import EventType
from zerg.events import event_bus
from zerg.managers.agent_runner import AgentRunner
from zerg.models.models import Agent
from zerg.models.models import NodeExecutionState
from zerg.models.models import Workflow
from zerg.models.models import WorkflowExecution
from zerg.tools.unified_access import get_tool_resolver

logger = logging.getLogger(__name__)


class WorkflowExecutionEngine:
    """Synchronous *linear* workflow execution.

    Upcoming milestones will add:
    • DAG traversal & dependency resolution
    • Parallel execution of independent branches
    • Real execution of tool / trigger / agent nodes
    • Streaming progress over the EventBus so WebSocket clients receive live
      updates without polling.
    """

    # ------------------------------------------------------------------
    # Public API – called from FastAPI routers
    # ------------------------------------------------------------------

    async def execute_workflow(
        self, workflow_id: int, trigger_type: str = "manual", trigger_config: Dict[str, Any] = None
    ) -> int:
        """Run *workflow_id* and return the new `WorkflowExecution.id`.

        We off-load the blocking DB / CPU part to a thread-pool via
        ``asyncio.to_thread`` so the coroutine does not block the event loop.
        The function is therefore safe to ``await`` directly from a request
        handler (no `BackgroundTasks` yet – that can come later).

        Args:
            workflow_id: The ID of the workflow to execute
            trigger_type: Type of trigger ("manual", "schedule", "webhook", etc.)
            trigger_config: Additional config from the trigger that initiated this execution
        """

        logger.info("[WorkflowEngine] queued execution – workflow_id=%s trigger_type=%s", workflow_id, trigger_type)

        session_factory = get_session_factory()

        async def _run() -> int:  # runs inside a worker thread
            with session_factory() as db:
                return await self._run_async(db, workflow_id, trigger_type, trigger_config or {})

        execution_id: int = await _run()
        logger.info("[WorkflowEngine] execution finished – workflow_id=%s execution_id=%s", workflow_id, execution_id)
        return execution_id

    # ------------------------------------------------------------------
    # Internal helpers – executed in worker thread
    # ------------------------------------------------------------------

    async def _run_async(
        self, db: Session, workflow_id: int, trigger_type: str = "manual", trigger_config: Dict[str, Any] = None
    ) -> int:
        """Asynchronous implementation for real node execution."""

        workflow: Workflow | None = db.query(Workflow).filter_by(id=workflow_id, is_active=True).first()
        if workflow is None:
            logger.error("[WorkflowEngine] workflow not found or inactive – id=%s", workflow_id)
            raise ValueError("Workflow not found or inactive")

        # ------------------------------------------------------------------
        # 1) Create parent WorkflowExecution row
        # ------------------------------------------------------------------
        execution = WorkflowExecution(
            workflow_id=workflow_id,
            status="running",
            started_at=datetime.now(timezone.utc),
            triggered_by=trigger_type,  # Track how this execution was triggered
        )
        db.add(execution)
        db.commit()

        # Final summary line
        self._publish_node_log(
            execution_id=execution.id,
            node_id="workflow",
            stream="stdout",
            text="Workflow execution completed successfully",
        )
        db.refresh(execution)

        log_lines: list[str] = []

        # ------------------------------------------------------------------
        # 2) Execute workflow using DAG traversal
        # ------------------------------------------------------------------
        canvas: Dict[str, Any] = workflow.canvas_data or {}
        nodes: list[Dict[str, Any]] = canvas.get("nodes", [])
        edges: list[Dict[str, Any]] = canvas.get("edges", [])

        # Build DAG and execute
        try:
            await self._execute_dag(db, execution, nodes, edges, log_lines, canvas)
        except Exception as e:
            # DAG execution failed - mark workflow as failed
            execution.status = "failed"
            execution.error = str(e)
            execution.finished_at = datetime.now(timezone.utc)
            execution.log = "\n".join(log_lines)
            db.commit()

            self._publish_execution_finished(
                execution_id=execution.id,
                status="failed",
                error=str(e),
                duration_ms=self._duration_ms(execution),
            )
            return execution.id

        # ------------------------------------------------------------------
        # 3) Success path – but honour user cancellation that may have been set
        #    after the last node finished processing.
        # ------------------------------------------------------------------

        db.refresh(execution)

        if execution.status == "cancelled":
            # User cancelled while last node was running – respect it.
            execution.finished_at = datetime.now(timezone.utc)
            execution.log = "\n".join(log_lines)
            db.commit()

            self._publish_execution_finished(
                execution_id=execution.id,
                status="cancelled",
                error=getattr(execution, "cancel_reason", None),
                duration_ms=self._duration_ms(execution),
            )
            logger.info("[WorkflowEngine] execution %s cancelled after last node", execution.id)
            return execution.id

        # Otherwise mark success
        execution.status = "success"
        execution.finished_at = datetime.now(timezone.utc)
        execution.log = "\n".join(log_lines)
        db.commit()

        self._publish_execution_finished(
            execution_id=execution.id,
            status="success",
            error=None,
            duration_ms=self._duration_ms(execution),
        )

        return execution.id

    # ------------------------------------------------------------------
    # DAG Execution Engine
    # ------------------------------------------------------------------

    async def _execute_dag(
        self,
        db: Session,
        execution: WorkflowExecution,
        nodes: list[Dict[str, Any]],
        edges: list[Dict[str, Any]],
        log_lines: list[str],
        canvas: Dict[str, Any] = None,
    ) -> None:
        """Execute workflow nodes using DAG traversal with dependency resolution."""

        # Build dependency graph
        node_deps = self._build_dependency_graph(nodes, edges)
        node_outputs = {}  # Store outputs from completed nodes
        completed_nodes = set()
        running_tasks = {}  # Track currently running node tasks

        # Create all node states upfront
        node_states = {}
        for node in nodes:
            node_id = str(node.get("id", "unknown"))
            node_state = NodeExecutionState(
                workflow_execution_id=execution.id,
                node_id=node_id,
                status="idle",
            )
            db.add(node_state)
            node_states[node_id] = node_state

        db.commit()

        # Main execution loop
        while len(completed_nodes) < len(nodes):
            # Check for cancellation
            db.refresh(execution)
            if execution.status == "cancelled":
                # Cancel all running tasks
                for task in running_tasks.values():
                    task.cancel()

                log_lines.append("Execution cancelled by user")
                execution.finished_at = datetime.now(timezone.utc)
                execution.log = "\n".join(log_lines)
                db.commit()

                self._publish_execution_finished(
                    execution_id=execution.id,
                    status="cancelled",
                    error=getattr(execution, "cancel_reason", "cancelled"),
                    duration_ms=self._duration_ms(execution),
                )
                return

            # Find nodes ready to execute (dependencies satisfied)
            ready_nodes = []
            for node in nodes:
                node_id = str(node.get("id", "unknown"))
                if (
                    node_id not in completed_nodes
                    and node_id not in running_tasks
                    and self._node_dependencies_satisfied(node_id, node_deps, completed_nodes)
                ):
                    ready_nodes.append(node)

            # Start execution of ready nodes
            for node in ready_nodes:
                node_id = str(node.get("id", "unknown"))
                # Add canvas context to node for retry configuration
                enhanced_node = dict(node)
                enhanced_node["_canvas_context"] = canvas or {}

                task = asyncio.create_task(
                    self._execute_node_with_context(db, enhanced_node, execution.id, node_outputs, node_states[node_id])
                )
                running_tasks[node_id] = task

                # Mark as running
                node_states[node_id].status = "running"
                db.commit()

                self._publish_node_event(
                    execution_id=execution.id,
                    node_id=node_id,
                    status="running",
                    output=None,
                    error=None,
                )

            # Wait for at least one task to complete
            if running_tasks:
                done, pending = await asyncio.wait(running_tasks.values(), return_when=asyncio.FIRST_COMPLETED)

                # Process completed tasks
                for task in done:
                    # Find which node this task belongs to
                    completed_node_id = None
                    for nid, t in running_tasks.items():
                        if t == task:
                            completed_node_id = nid
                            break

                    if completed_node_id:
                        try:
                            output = await task
                            # Node succeeded
                            node_outputs[completed_node_id] = output
                            completed_nodes.add(completed_node_id)
                            node_states[completed_node_id].status = "success"
                            node_states[completed_node_id].output = output
                            db.commit()

                            self._publish_node_event(
                                execution_id=execution.id,
                                node_id=completed_node_id,
                                status="success",
                                output=output,
                                error=None,
                            )

                            log_lines.append(f"Node {completed_node_id} completed successfully")

                        except Exception as e:
                            # Node failed - mark workflow as failed
                            node_states[completed_node_id].status = "failed"
                            node_states[completed_node_id].error = str(e)
                            db.commit()

                            self._publish_node_event(
                                execution_id=execution.id,
                                node_id=completed_node_id,
                                status="failed",
                                output=None,
                                error=str(e),
                            )

                            log_lines.append(f"Node {completed_node_id} failed: {e}")
                            raise  # Re-raise to fail the entire workflow

                        # Remove from running tasks
                        del running_tasks[completed_node_id]
            else:
                # No tasks running and no ready nodes - check for deadlock
                if len(completed_nodes) < len(nodes):
                    remaining_nodes = [n for n in nodes if str(n.get("id", "unknown")) not in completed_nodes]
                    raise RuntimeError(
                        f"Workflow deadlock: remaining nodes {[n.get('id') for n in remaining_nodes]} cannot execute"
                    )
                break

        # All nodes completed successfully
        log_lines.append(f"DAG execution completed - {len(completed_nodes)} nodes executed")

    def _build_dependency_graph(self, nodes: list[Dict[str, Any]], edges: list[Dict[str, Any]]) -> Dict[str, set[str]]:
        """Build a dependency graph from nodes and edges."""

        # Initialize empty dependency sets for all nodes
        deps = {}
        for node in nodes:
            node_id = str(node.get("id", "unknown"))
            deps[node_id] = set()

        # Add dependencies based on edges
        for edge in edges:
            source = str(edge.get("source", ""))
            target = str(edge.get("target", ""))

            if source and target and target in deps:
                deps[target].add(source)

        return deps

    def _node_dependencies_satisfied(
        self, node_id: str, node_deps: Dict[str, set[str]], completed_nodes: set[str]
    ) -> bool:
        """Check if all dependencies for a node are satisfied."""

        required_deps = node_deps.get(node_id, set())
        return required_deps.issubset(completed_nodes)

    async def _execute_node_with_context(
        self,
        db: Session,
        node: Dict[str, Any],
        execution_id: int,
        node_outputs: Dict[str, Any],
        node_state: NodeExecutionState,
    ) -> Dict[str, Any]:
        """Execute a node with access to outputs from previous nodes and retry logic."""

        node_id = str(node.get("id", "unknown"))
        node_type = str(node.get("type", "unknown"))

        # Get retry configuration - need to get canvas from parent execution context
        # For now, we'll extract from the global context or use defaults
        canvas = node.get("_canvas_context", {})
        policy: Dict[str, Any] = (canvas.get("retries") or {}) if isinstance(canvas.get("retries"), dict) else {}
        default_max_retries: int = int(policy.get("default", 0))
        backoff_strategy: str = str(policy.get("backoff", "exp"))

        # Per-node override (optional)
        node_policy: Dict[str, Any] = node.get("retries", {}) if isinstance(node.get("retries"), dict) else {}
        max_retries: int = int(node_policy.get("max", default_max_retries))

        # Add context from previous nodes to the node data
        enhanced_node = dict(node)
        enhanced_node["context"] = node_outputs

        attempt = 0

        while attempt <= max_retries:
            try:
                # Execute the node with context
                output = await self._execute_node(db, enhanced_node, node_type, execution_id, attempt)

                # Log success
                self._publish_node_log(
                    execution_id=execution_id,
                    node_id=node_id,
                    stream="stdout",
                    text=f"Node {node_id} succeeded (attempt {attempt})",
                )

                return output

            except Exception as exc:
                if attempt < max_retries:
                    # Log retry
                    self._publish_node_event(
                        execution_id=execution_id,
                        node_id=node_id,
                        status="retrying",
                        output=None,
                        error=str(exc),
                    )

                    self._publish_node_log(
                        execution_id=execution_id,
                        node_id=node_id,
                        stream="stdout",
                        text=f"RETRY {attempt + 1}/{max_retries}: {exc}",
                    )

                    # Wait with backoff
                    backoff_sec = self._calc_backoff_delay(backoff_strategy, attempt)
                    await asyncio.sleep(backoff_sec)
                    attempt += 1
                    continue

                # Max retries exceeded
                self._publish_node_log(
                    execution_id=execution_id,
                    node_id=node_id,
                    stream="stderr",
                    text=f"Node {node_id} failed after {attempt + 1} attempt(s): {exc}",
                )

                logger.exception("[WorkflowEngine] node execution failed – node_id=%s", node_id)
                raise

    # ------------------------------------------------------------------
    # Event helper
    # ------------------------------------------------------------------

    def _publish_node_event(
        self,
        *,
        execution_id: int,
        node_id: str,
        status: str,
        output: Dict[str, Any] | None,
        error: str | None,
    ) -> None:
        """Publish NODE_STATE_CHANGED via the global event bus (sync helper)."""

        payload = {
            "execution_id": execution_id,
            "node_id": node_id,
            "status": status,
            "output": output,
            "error": error,
            "event_type": EventType.NODE_STATE_CHANGED,
        }

        # Run publish in a fresh event loop inside the worker thread.
        # Since we're now async, create a task instead of trying to run a new loop
        try:
            loop = asyncio.get_running_loop()
            loop.create_task(event_bus.publish(EventType.NODE_STATE_CHANGED, payload))
            # Don't await - let it run in background
        except RuntimeError:
            # No running loop, create one
            loop = asyncio.new_event_loop()
            asyncio.set_event_loop(loop)
            try:
                loop.run_until_complete(event_bus.publish(EventType.NODE_STATE_CHANGED, payload))
            finally:
                loop.close()

    # ------------------------------------------------------------------
    # Execution finished helper
    # ------------------------------------------------------------------

    def _publish_execution_finished(
        self,
        *,
        execution_id: int,
        status: str,
        error: str | None,
        duration_ms: int | None,
    ) -> None:
        """Publish EXECUTION_FINISHED event for listeners (WebSocket)."""

        payload = {
            "execution_id": execution_id,
            "status": status,
            "error": error,
            "duration_ms": duration_ms,
            "event_type": EventType.EXECUTION_FINISHED,
        }

        try:
            loop = asyncio.get_running_loop()
            loop.create_task(event_bus.publish(EventType.EXECUTION_FINISHED, payload))
            # Don't await - let it run in background
        except RuntimeError:
            # No running loop, create one
            loop = asyncio.new_event_loop()
            asyncio.set_event_loop(loop)
            try:
                loop.run_until_complete(event_bus.publish(EventType.EXECUTION_FINISHED, payload))
            finally:
                loop.close()

    # ------------------------------------------------------------------
    # Node log helper – stream stdout/stderr lines
    # ------------------------------------------------------------------

    def _publish_node_log(
        self,
        *,
        execution_id: int,
        node_id: str,
        stream: str,
        text: str,
    ) -> None:
        """Publish NODE_LOG event carrying a single log line."""

        payload = {
            "execution_id": execution_id,
            "node_id": node_id,
            "stream": stream,
            "text": text,
            "event_type": EventType.NODE_LOG,
        }

        try:
            loop = asyncio.get_running_loop()
            loop.create_task(event_bus.publish(EventType.NODE_LOG, payload))
            # Don't await - let it run in background
        except RuntimeError:
            # No running loop, create one
            loop = asyncio.new_event_loop()
            asyncio.set_event_loop(loop)
            try:
                loop.run_until_complete(event_bus.publish(EventType.NODE_LOG, payload))
            finally:
                loop.close()

    @staticmethod
    def _duration_ms(execution: WorkflowExecution) -> int | None:
        if execution.started_at and execution.finished_at:
            delta = execution.finished_at - execution.started_at
            return int(delta.total_seconds() * 1000)
        return None

    # ------------------------------------------------------------------
    # Internal util – placeholder node execution & back-off calculation
    # ------------------------------------------------------------------

    async def _execute_node(
        self, db: Session, node: Dict[str, Any], node_type: str, execution_id: int, attempt: int = 0
    ) -> Dict[str, Any]:
        """Execute a node based on its type and return the output."""

        node_id = str(node.get("id", "unknown"))

        # Log node execution start
        self._publish_node_log(
            execution_id=execution_id,
            node_id=node_id,
            stream="stdout",
            text=f"Starting {node_type} node execution",
        )

        if node_type.lower() == "tool":
            return await self._execute_tool_node(db, node, execution_id)
        elif node_type.lower() == "agent":
            return await self._execute_agent_node(db, node, execution_id)
        elif node_type.lower() == "trigger":
            return await self._execute_trigger_node(db, node, execution_id)
        else:
            # Fall back to placeholder for unknown node types
            # Use original node data for simulated failures (not the enhanced context)
            original_node = dict(node)
            if "context" in original_node:
                del original_node["context"]
            if "_canvas_context" in original_node:
                del original_node["_canvas_context"]

            self._execute_placeholder_node(node_type, original_node)
            return {"result": f"{node_type}_executed", "type": "placeholder", "attempt": attempt}

    async def _execute_tool_node(self, db: Session, node: Dict[str, Any], execution_id: int) -> Dict[str, Any]:
        """Execute a tool node using the tool registry."""

        node_id = str(node.get("id", "unknown"))
        tool_name = node.get("tool_name") or node.get("name", "")
        tool_params = node.get("parameters", {})

        if not tool_name:
            raise ValueError("Tool node missing tool_name field")

        # Get tool using unified resolver - supports builtin, registered, and MCP tools
        resolver = get_tool_resolver()

        # Use the unified resolver for efficient tool lookup
        tool = resolver.get_tool(tool_name)

        if not tool:
            available_tools = resolver.get_tool_names()
            raise ValueError(f"Tool '{tool_name}' not found. Available: {available_tools}")

        self._publish_node_log(
            execution_id=execution_id,
            node_id=node_id,
            stream="stdout",
            text=f"Executing tool: {tool_name}",
        )

        try:
            # Execute the tool
            result = await tool.ainvoke(tool_params)

            self._publish_node_log(
                execution_id=execution_id,
                node_id=node_id,
                stream="stdout",
                text=f"Tool {tool_name} completed successfully",
            )

            return {"tool_name": tool_name, "parameters": tool_params, "result": result, "type": "tool"}

        except Exception as e:
            self._publish_node_log(
                execution_id=execution_id,
                node_id=node_id,
                stream="stderr",
                text=f"Tool {tool_name} failed: {str(e)}",
            )
            raise

    async def _execute_agent_node(self, db: Session, node: Dict[str, Any], execution_id: int) -> Dict[str, Any]:
        """Execute an agent node using AgentRunner."""

        node_id = str(node.get("id", "unknown"))
        agent_id = node.get("agent_id")

        if not agent_id:
            raise ValueError("Agent node missing agent_id field")

        # Get agent from database
        agent = db.query(Agent).filter_by(id=agent_id).first()
        if not agent:
            raise ValueError(f"Agent with id {agent_id} not found")

        self._publish_node_log(
            execution_id=execution_id,
            node_id=node_id,
            stream="stdout",
            text=f"Executing agent: {agent.name}",
        )

        try:
            # Create a temporary thread for this workflow execution
            thread = crud.create_thread(
                db=db,
                user_id=1,  # TODO: Get actual user from context
                thread_type="workflow",
                title=f"Workflow execution {execution_id}",
            )

            # Add user message with context from previous nodes
            user_message = node.get("message", "Execute this task")
            crud.create_message(db=db, thread_id=thread.id, role="user", content=user_message, processed=False)

            # Run the agent
            runner = AgentRunner(agent)
            created_messages = await runner.run_thread(db, thread)

            # Extract the assistant response
            assistant_messages = [msg for msg in created_messages if msg.role == "assistant"]
            result = assistant_messages[-1].content if assistant_messages else "No response generated"

            self._publish_node_log(
                execution_id=execution_id,
                node_id=node_id,
                stream="stdout",
                text=f"Agent {agent.name} completed successfully",
            )

            return {
                "agent_id": agent_id,
                "agent_name": agent.name,
                "message": user_message,
                "response": result,
                "type": "agent",
            }

        except Exception as e:
            self._publish_node_log(
                execution_id=execution_id,
                node_id=node_id,
                stream="stderr",
                text=f"Agent {agent.name} failed: {str(e)}",
            )
            raise

    async def _execute_trigger_node(self, db: Session, node: Dict[str, Any], execution_id: int) -> Dict[str, Any]:
        """Execute a trigger node by creating/configuring triggers."""

        node_id = str(node.get("id", "unknown"))
        trigger_type = node.get("trigger_type", "webhook")
        target_agent_id = node.get("target_agent_id")

        self._publish_node_log(
            execution_id=execution_id,
            node_id=node_id,
            stream="stdout",
            text=f"Processing {trigger_type} trigger for agent {target_agent_id}",
        )

        if not target_agent_id:
            raise ValueError("Trigger node missing target_agent_id field")

        # Create the trigger based on type
        if trigger_type == "webhook":
            return await self._create_webhook_trigger(db, node, target_agent_id, execution_id)
        elif trigger_type == "schedule":
            return await self._create_schedule_trigger(db, node, target_agent_id, execution_id)
        elif trigger_type == "email":
            return await self._create_email_trigger(db, node, target_agent_id, execution_id)
        else:
            raise ValueError(f"Unsupported trigger type: {trigger_type}")

    async def _create_webhook_trigger(
        self, db: Session, node: Dict[str, Any], agent_id: int, execution_id: int
    ) -> Dict[str, Any]:
        """Create a webhook trigger."""

        from zerg.crud import crud

        node_id = str(node.get("id", "unknown"))

        # Create the trigger
        trigger = crud.create_trigger(db=db, agent_id=agent_id, trigger_type="webhook", config=node.get("config", {}))

        # Generate webhook URL
        webhook_url = f"/api/triggers/{trigger.id}/events"

        self._publish_node_log(
            execution_id=execution_id,
            node_id=node_id,
            stream="stdout",
            text=f"Created webhook trigger {trigger.id} for agent {agent_id}",
        )

        return {
            "trigger_id": trigger.id,
            "trigger_type": "webhook",
            "webhook_url": webhook_url,
            "secret": trigger.secret,
            "status": "active",
            "type": "trigger",
        }

    async def _create_schedule_trigger(
        self, db: Session, node: Dict[str, Any], agent_id: int, execution_id: int
    ) -> Dict[str, Any]:
        """Create a scheduled trigger for workflow execution."""

        node_id = str(node.get("id", "unknown"))
        cron_expression = node.get("cron_expression")

        # Check if this is for workflow scheduling or agent scheduling
        schedule_type = node.get("schedule_type", "workflow")  # "workflow" or "agent"

        if not cron_expression:
            raise ValueError("Schedule trigger missing cron_expression field")

        if schedule_type == "workflow":
            # Schedule the current workflow to run on this cron schedule
            workflow_execution = db.query(WorkflowExecution).filter_by(id=execution_id).first()
            if not workflow_execution:
                raise ValueError(f"Workflow execution {execution_id} not found")

            workflow_id = workflow_execution.workflow_id

            # Import and use the workflow scheduler
            from zerg.services.workflow_scheduler import workflow_scheduler

            success = await workflow_scheduler.schedule_workflow(
                workflow_id=workflow_id, cron_expression=cron_expression, trigger_config=node.copy()
            )

            if success:
                self._publish_node_log(
                    execution_id=execution_id,
                    node_id=node_id,
                    stream="stdout",
                    text=f"Scheduled workflow {workflow_id} with cron: {cron_expression}",
                )

                return {
                    "workflow_id": workflow_id,
                    "trigger_type": "schedule",
                    "cron_expression": cron_expression,
                    "status": "scheduled",
                    "type": "trigger",
                }
            else:
                raise RuntimeError(f"Failed to schedule workflow {workflow_id}")

        else:
            # Original agent scheduling logic
            from zerg.crud import crud

            # Update the target agent's schedule
            agent = crud.get_agent(db, agent_id)
            if not agent:
                raise ValueError(f"Agent {agent_id} not found")

            # Update agent with schedule
            crud.update_agent(db, agent_id, schedule=cron_expression)

            # Try to register with scheduler if available
            try:
                import importlib.util

                if importlib.util.find_spec("zerg.services.scheduler_service") is not None:
                    # Scheduler service is available - could import and use it here
                    # For now, we'll just log that the schedule was set
                    self._publish_node_log(
                        execution_id=execution_id,
                        node_id=node_id,
                        stream="stdout",
                        text=f"Scheduled agent {agent_id} with cron: {cron_expression}",
                    )
                else:
                    raise ImportError("Scheduler service not found")
            except ImportError:
                # Scheduler service not available, just update the database
                self._publish_node_log(
                    execution_id=execution_id,
                    node_id=node_id,
                    stream="stdout",
                    text=f"Updated agent {agent_id} schedule (scheduler service not available)",
                )

            return {
                "agent_id": agent_id,
                "trigger_type": "schedule",
                "cron_expression": cron_expression,
                "status": "scheduled",
                "type": "trigger",
            }

    async def _create_email_trigger(
        self, db: Session, node: Dict[str, Any], agent_id: int, execution_id: int
    ) -> Dict[str, Any]:
        """Create an email trigger."""

        from zerg.crud import crud

        node_id = str(node.get("id", "unknown"))
        provider = node.get("email_provider", "gmail")
        filters = node.get("email_filters", {})

        config = {"provider": provider, "filters": filters}

        # Create email trigger
        trigger = crud.create_trigger(db=db, agent_id=agent_id, trigger_type="email", config=config)

        # TODO: Initialize provider-specific setup (Gmail watch, etc.)
        # This would require Gmail API integration which is complex

        self._publish_node_log(
            execution_id=execution_id,
            node_id=node_id,
            stream="stdout",
            text=f"Created {provider} email trigger {trigger.id} for agent {agent_id}",
        )

        return {
            "trigger_id": trigger.id,
            "trigger_type": "email",
            "provider": provider,
            "filters": filters,
            "status": "created",
            "type": "trigger",
        }

    @staticmethod
    def _execute_placeholder_node(node_type: str, node_payload: Dict[str, Any]) -> None:
        """Simulate node execution with optional *simulate_failures* logic.

        This keeps the engine fully deterministic for unit tests **before**
        real tool/agent execution lands.  Callers can set

        ```json
        { "type": "dummy", "simulate_failures": 2 }
        ```

        to raise an exception *simulate_failures* times before succeeding.
        The counter is kept *inside* the node payload so repeated calls within
        one engine execution see the updated count.
        """

        # Simulated delay
        time.sleep(0.005)

        remaining_failures = int(node_payload.get("simulate_failures", 0))
        if remaining_failures > 0:
            # Decrement counter so next retry may succeed
            node_payload["simulate_failures"] = remaining_failures - 1
            raise RuntimeError("Simulated node failure for testing")

    @staticmethod
    def _calc_backoff_delay(strategy: str, attempt: int) -> float:
        """Return back-off seconds based on *strategy* and *attempt* (0-based)."""

        strategy = strategy.lower()
        if strategy == "linear":
            return 0.1 * (attempt + 1)
        # default exponential
        return 0.1 * (2**attempt)


# ---------------------------------------------------------------------------
# Public singleton – imported by routers
# ---------------------------------------------------------------------------

workflow_execution_engine = WorkflowExecutionEngine()
