"""
Simplified Workflow Execution Engine

Clean, focused workflow execution using WorkflowData schema.
~150 lines vs the previous 600+ line monolith.
"""

import logging
from typing import Any
from typing import Dict
from typing import List
from typing import TypedDict
from typing import Union

from langgraph.checkpoint.memory import MemorySaver
from langgraph.graph import END
from langgraph.graph import START
from langgraph.graph import StateGraph

from zerg.database import get_session_factory
from zerg.events import EventType
from zerg.events.publisher import publish_event
from zerg.models.enums import FailureKind
from zerg.models.models import Workflow
from zerg.models.models import WorkflowExecution
from zerg.schemas.workflow import WorkflowData
from zerg.services.execution_state import ExecutionStateMachine
from zerg.services.node_executors import create_node_executor
from zerg.utils.time import utc_now_naive

logger = logging.getLogger(__name__)


class WorkflowState(TypedDict):
    """State passed between nodes in the workflow."""

    execution_id: int
    node_outputs: Dict[str, Any]
    completed_nodes: List[str]
    error: Union[str, None]


class WorkflowEngine:
    """Simplified workflow engine using WorkflowData schema."""

    async def execute_workflow(self, workflow_id: int, trigger_type: str = "manual") -> int:
        """Execute workflow and return execution ID."""
        logger.info(f"[WorkflowEngine] Starting execution – workflow_id={workflow_id}")

        session_factory = get_session_factory()
        with session_factory() as db:
            # Create execution record in WAITING state
            execution = WorkflowExecution(
                workflow_id=workflow_id,
                started_at=utc_now_naive(),
                triggered_by=trigger_type,
            )
            db.add(execution)
            db.commit()

            try:
                # Mark as running using state machine
                ExecutionStateMachine.mark_running(execution)
                db.commit()

                await self._execute_workflow_internal(workflow_id, execution, db)
                return execution.id
            except Exception as e:
                # Mark as failed using state machine only if not already finished
                if ExecutionStateMachine.can_finish(execution):
                    ExecutionStateMachine.mark_failure(execution, error_message=str(e), failure_kind=FailureKind.SYSTEM)
                    execution.finished_at = utc_now_naive()
                    db.commit()

                await self._publish_execution_finished(
                    execution_id=execution.id, execution=execution, duration_ms=self._duration_ms(execution)
                )
                logger.exception(f"[WorkflowEngine] Execution failed – execution_id={execution.id}")
                raise

    async def _execute_workflow_internal(self, workflow_id: int, execution: WorkflowExecution, db):
        """Load and execute workflow."""
        # Load workflow
        workflow_model = db.query(Workflow).filter_by(id=workflow_id, is_active=True).first()
        if not workflow_model:
            raise ValueError("Workflow not found or inactive")

        try:
            workflow_data = WorkflowData(**workflow_model.canvas)
            logger.info(f"[WorkflowEngine] Loaded workflow with {len(workflow_data.nodes)} nodes")
        except Exception as e:
            raise ValueError(f"Invalid workflow data: {e}")

        # Handle empty workflows
        if not workflow_data.nodes:
            logger.info(f"[WorkflowEngine] Empty workflow {workflow_id} - completing immediately")
            ExecutionStateMachine.mark_success(execution)
            execution.finished_at = utc_now_naive()
            db.commit()
            await publish_event(
                EventType.EXECUTION_FINISHED,
                {
                    "execution_id": execution.id,
                    "workflow_id": workflow_id,
                    "status": ExecutionStateMachine.get_display_label(execution),
                    "event_type": EventType.EXECUTION_FINISHED,
                },
            )
            return

        # Build and execute graph
        graph = self._build_langgraph(workflow_data, execution.id)
        await self._execute_graph(graph, execution, db, workflow_id)

    def _build_langgraph(self, workflow_data: WorkflowData, execution_id: int):
        """Build LangGraph from WorkflowData."""
        graph = StateGraph(WorkflowState)

        # Add nodes
        for node in workflow_data.nodes:
            executor = create_node_executor(node, self._publish_node_event)
            graph.add_node(node.id, executor.execute)

        # Find start and end nodes
        target_nodes = {edge.to_node_id for edge in workflow_data.edges}
        source_nodes = {edge.from_node_id for edge in workflow_data.edges}

        start_nodes = [node.id for node in workflow_data.nodes if node.id not in target_nodes]
        end_nodes = [node.id for node in workflow_data.nodes if node.id not in source_nodes]

        # Connect graph
        for start_node in start_nodes:
            graph.add_edge(START, start_node)

        # Group edges by source node to handle conditional routing
        edges_by_source = {}
        for edge in workflow_data.edges:
            if edge.from_node_id not in edges_by_source:
                edges_by_source[edge.from_node_id] = []
            edges_by_source[edge.from_node_id].append(edge)

        # Add edges with conditional routing support
        for source_node_id, edges in edges_by_source.items():
            from_node = next((n for n in workflow_data.nodes if n.id == source_node_id), None)

            if from_node and from_node.type == "conditional":
                # For conditional nodes, create a router that handles all outgoing edges
                true_targets = [e.to_node_id for e in edges if e.config.get("branch", "true") == "true"]
                false_targets = [e.to_node_id for e in edges if e.config.get("branch", "true") == "false"]

                def make_conditional_router(true_list, false_list):
                    def conditional_router(state):
                        """Route based on conditional node result."""
                        if source_node_id in state["node_outputs"]:
                            conditional_result = state["node_outputs"][source_node_id]

                            # Envelope format only
                            if isinstance(conditional_result, dict) and "value" in conditional_result:
                                result_value = conditional_result["value"]
                                if isinstance(result_value, dict) and "branch" in result_value:
                                    branch = result_value["branch"]
                                else:
                                    branch = "false"
                            else:
                                branch = "false"

                            if branch == "true" and true_list:
                                return true_list[0]  # Take first true target
                            elif branch == "false" and false_list:
                                return false_list[0]  # Take first false target
                        return END  # End workflow if no valid route

                    return conditional_router

                # Build the routing map
                route_map = {}
                if true_targets:
                    route_map[true_targets[0]] = true_targets[0]
                if false_targets:
                    route_map[false_targets[0]] = false_targets[0]
                route_map[END] = END

                graph.add_conditional_edges(
                    source_node_id, make_conditional_router(true_targets, false_targets), route_map
                )
            else:
                # Regular edges - add them normally
                for edge in edges:
                    graph.add_edge(edge.from_node_id, edge.to_node_id)

        for end_node in end_nodes:
            graph.add_edge(end_node, END)

        checkpointer = MemorySaver()
        return graph.compile(checkpointer=checkpointer)

    async def _execute_graph(self, graph, execution: WorkflowExecution, db, workflow_id: int):
        """Execute the compiled graph."""
        initial_state = {
            "execution_id": execution.id,
            "node_outputs": {},
            "completed_nodes": [],
            "error": None,
        }

        config = {"configurable": {"thread_id": f"workflow_{execution.id}"}}
        logger.info(f"[WorkflowEngine] Starting streaming execution – workflow_id={workflow_id}")

        # Stream execution for real-time updates
        async for chunk in graph.astream(initial_state, config):
            if chunk:
                for node_id, state_update in chunk.items():
                    if state_update and hasattr(state_update, "get"):
                        await self._publish_streaming_progress(
                            execution_id=execution.id,
                            completed_nodes=state_update.get("completed_nodes", []),
                            node_outputs=state_update.get("node_outputs", {}),
                            error=state_update.get("error"),
                        )

        # Mark as successful using state machine
        ExecutionStateMachine.mark_success(execution)
        execution.finished_at = utc_now_naive()
        db.commit()

        await self._publish_execution_finished(
            execution_id=execution.id, execution=execution, duration_ms=self._duration_ms(execution)
        )
        logger.info(f"[WorkflowEngine] Execution completed – execution_id={execution.id}")

    # Event publishing methods
    async def _publish_node_event(self, *, execution_id: int, node_id: str, node_state, output: Any):
        # Clean Phase/Result architecture - no legacy fields
        await publish_event(
            EventType.NODE_STATE_CHANGED,
            {
                "execution_id": execution_id,
                "node_id": node_id,
                "phase": node_state.phase,
                "result": node_state.result,
                "attempt_no": node_state.attempt_no,
                "failure_kind": node_state.failure_kind,
                "error_message": node_state.error_message,
                "output": output,
                "event_type": EventType.NODE_STATE_CHANGED,
            },
        )

    async def _publish_streaming_progress(
        self, *, execution_id: int, completed_nodes: List[str], node_outputs: Dict[str, Any], error: str
    ):
        await publish_event(
            EventType.WORKFLOW_PROGRESS,
            {
                "execution_id": execution_id,
                "completed_nodes": completed_nodes,
                "node_outputs": node_outputs,
                "error": error,
                "event_type": EventType.WORKFLOW_PROGRESS,
            },
        )

    async def _publish_execution_finished(self, *, execution_id: int, execution: WorkflowExecution, duration_ms: int):
        # Clean Phase/Result architecture - no legacy fields
        await publish_event(
            EventType.EXECUTION_FINISHED,
            {
                "execution_id": execution_id,
                "result": execution.result,
                "attempt_no": execution.attempt_no,
                "failure_kind": execution.failure_kind,
                "error_message": execution.error_message,
                "duration_ms": duration_ms,
                "event_type": EventType.EXECUTION_FINISHED,
            },
        )

    @staticmethod
    def _duration_ms(execution: WorkflowExecution) -> Union[int, None]:
        if execution.started_at and execution.finished_at:
            delta = execution.finished_at - execution.started_at
            return int(delta.total_seconds() * 1000)
        return None


# Singleton instance
workflow_engine = WorkflowEngine()
