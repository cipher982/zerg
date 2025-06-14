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

from zerg.database import get_session_factory
from zerg.events import EventType
from zerg.events import event_bus
from zerg.models.models import NodeExecutionState
from zerg.models.models import Workflow
from zerg.models.models import WorkflowExecution

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

    async def execute_workflow(self, workflow_id: int) -> int:
        """Run *workflow_id* and return the new `WorkflowExecution.id`.

        We off-load the blocking DB / CPU part to a thread-pool via
        ``asyncio.to_thread`` so the coroutine does not block the event loop.
        The function is therefore safe to ``await`` directly from a request
        handler (no `BackgroundTasks` yet – that can come later).
        """

        logger.info("[WorkflowEngine] queued execution – workflow_id=%s", workflow_id)

        session_factory = get_session_factory()

        def _run() -> int:  # runs inside a worker thread
            with session_factory() as db:
                return self._run_sync(db, workflow_id)

        execution_id: int = await asyncio.to_thread(_run)
        logger.info("[WorkflowEngine] execution finished – workflow_id=%s execution_id=%s", workflow_id, execution_id)
        return execution_id

    # ------------------------------------------------------------------
    # Internal helpers – executed in worker thread
    # ------------------------------------------------------------------

    def _run_sync(self, db: Session, workflow_id: int) -> int:
        """Synchronous implementation – *must* be called within a worker thread."""

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
        )
        db.add(execution)
        db.commit()
        db.refresh(execution)

        log_lines: list[str] = []

        # ------------------------------------------------------------------
        # 2) Sequentially process every node (placeholder logic)
        # ------------------------------------------------------------------
        canvas: Dict[str, Any] = workflow.canvas_data or {}
        nodes: list[Dict[str, Any]] = canvas.get("nodes", [])

        for idx, node in enumerate(nodes):
            # Respect cancellation requested by user (status set to "cancelled").
            db.refresh(execution)
            if execution.status == "cancelled":
                log_lines.append("Execution cancelled by user – stopping before node %s" % idx)
                execution.finished_at = datetime.now(timezone.utc)
                execution.log = "\n".join(log_lines)
                db.commit()

                self._publish_execution_finished(
                    execution_id=execution.id,
                    status="cancelled",
                    error=getattr(execution, "cancel_reason", "cancelled"),
                    duration_ms=self._duration_ms(execution),
                )
                return execution.id
            node_id: str = str(node.get("id", f"idx_{idx}"))
            node_type: str = str(node.get("type", "unknown"))

            # -- persist initial NodeExecutionState (running) -------------
            node_state = NodeExecutionState(
                workflow_execution_id=execution.id,
                node_id=node_id,
                status="running",
            )
            db.add(node_state)
            db.commit()
            db.refresh(node_state)

            # Emit *running* event
            self._publish_node_event(
                execution_id=execution.id,
                node_id=node_id,
                status="running",
                output=None,
                error=None,
            )

            # ------------------------------------------------------------------
            # Retry / back-off handling (simple v1 implementation)
            # ------------------------------------------------------------------

            # Defaults from workflow-level policy
            policy: Dict[str, Any] = (canvas.get("retries") or {}) if isinstance(canvas.get("retries"), dict) else {}
            default_max_retries: int = int(policy.get("default", 0))
            backoff_strategy: str = str(policy.get("backoff", "exp"))

            # Per-node override (optional)
            node_policy: Dict[str, Any] = node.get("retries", {}) if isinstance(node.get("retries"), dict) else {}
            max_retries: int = int(node_policy.get("max", default_max_retries))

            attempt = 0
            last_exc: Exception | None = None

            while attempt <= max_retries:
                try:
                    # Placeholder execution – replace with real logic later
                    self._execute_placeholder_node(node_type, node)

                    # Mock output so the front-end has something to display.
                    output: Dict[str, Any] = {"result": f"{node_type}_executed", "attempt": attempt}

                    node_state.status = "success"
                    node_state.output = output
                    db.commit()

                    # Emit *success* event
                    self._publish_node_event(
                        execution_id=execution.id,
                        node_id=node_id,
                        status="success",
                        output=output,
                        error=None,
                    )

                    log_lines.append(f"Node {node_id} ({node_type}) executed – OK (attempt {attempt})")
                    break  # success

                except Exception as exc:  # noqa: BLE001 – placeholder for broad errors
                    last_exc = exc
                    if attempt < max_retries:
                        # Log + wait for back-off then retry
                        log_lines.append(
                            f"Node {node_id} ({node_type}) failed – RETRY {attempt+1}/{max_retries}: {exc}"
                        )
                        # Emit retrying state for UI
                        self._publish_node_event(
                            execution_id=execution.id,
                            node_id=node_id,
                            status="retrying",
                            output=None,
                            error=str(exc),
                        )

                        # sleep back-off (blocking since we are in worker thread)
                        backoff_sec = self._calc_backoff_delay(backoff_strategy, attempt)
                        time.sleep(backoff_sec)
                        attempt += 1
                        continue

                    # Max retries exceeded – mark failed
                    node_state.status = "failed"
                    node_state.error = str(exc)
                    db.commit()

                    # Emit *failed* event
                    self._publish_node_event(
                        execution_id=execution.id,
                        node_id=node_id,
                        status="failed",
                        output=None,
                        error=str(exc),
                    )

                    # -- mark parent execution failed ------------------------
                    execution.status = "failed"
                    execution.error = str(exc)
                    execution.finished_at = datetime.now(timezone.utc)
                    execution.log = "\n".join(log_lines)
                    db.commit()

                    # Emit execution_finished (failed)
                    self._publish_execution_finished(
                        execution_id=execution.id,
                        status="failed",
                        error=str(exc),
                        duration_ms=self._duration_ms(execution),
                    )
                    logger.exception("[WorkflowEngine] node execution failed – node_id=%s", node_id)
                    return execution.id

            # End while

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
    # Event helper
    # ------------------------------------------------------------------

    @staticmethod
    def _publish_node_event(
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
        try:
            asyncio.run(event_bus.publish(EventType.NODE_STATE_CHANGED, payload))
        except RuntimeError:
            # In case we're already inside a running loop (unlikely in thread)
            loop = asyncio.new_event_loop()
            loop.run_until_complete(event_bus.publish(EventType.NODE_STATE_CHANGED, payload))
            loop.close()

    # ------------------------------------------------------------------
    # Execution finished helper
    # ------------------------------------------------------------------

    @staticmethod
    def _publish_execution_finished(
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
            asyncio.run(event_bus.publish(EventType.EXECUTION_FINISHED, payload))
        except RuntimeError:
            loop = asyncio.new_event_loop()
            loop.run_until_complete(event_bus.publish(EventType.EXECUTION_FINISHED, payload))
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
