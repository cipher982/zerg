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
from datetime import datetime
from datetime import timezone
from typing import Any
from typing import Dict

from sqlalchemy.orm import Session

from zerg.database import get_session_factory
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

            try:
                # Placeholder execution – replace with real logic later
                # We still want a *tiny* sleep so tests see the status change
                # and developers can watch the UI progress.
                asyncio.run(asyncio.sleep(0.005))  # type: ignore[arg-type]

                # Mock output so the front-end has something to display.
                output: Dict[str, Any] = {"result": f"{node_type}_executed"}

                node_state.status = "success"
                node_state.output = output
                db.commit()

                log_lines.append(f"Node {node_id} ({node_type}) executed – OK")

            except Exception as exc:  # pylint: disable=broad-except
                # -- mark node failed -------------------------------------
                node_state.status = "failed"
                node_state.error = str(exc)
                db.commit()

                # -- mark parent execution failed ------------------------
                execution.status = "failed"
                execution.error = str(exc)
                execution.finished_at = datetime.now(timezone.utc)
                execution.log = "\n".join(log_lines)
                db.commit()
                logger.exception("[WorkflowEngine] node execution failed – node_id=%s", node_id)
                return execution.id

        # ------------------------------------------------------------------
        # 3) Success path – mark execution done
        # ------------------------------------------------------------------
        execution.status = "success"
        execution.finished_at = datetime.now(timezone.utc)
        execution.log = "\n".join(log_lines)
        db.commit()

        return execution.id


# ---------------------------------------------------------------------------
# Public singleton – imported by routers
# ---------------------------------------------------------------------------

workflow_execution_engine = WorkflowExecutionEngine()
