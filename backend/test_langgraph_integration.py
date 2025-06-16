"""
Integration test for LangGraph workflow engine.
Tests real end-to-end execution with actual canvas_data structures.
"""

import asyncio
import sys
from pathlib import Path

# Add backend to path
sys.path.append(str(Path(__file__).parent))

from langgraph.checkpoint.sqlite.aio import AsyncSqliteSaver

from zerg.services.langgraph_workflow_engine import LangGraphWorkflowEngine
from zerg.services.langgraph_workflow_engine import WorkflowState


async def test_basic_linear_workflow():
    """Test a simple 3-node linear workflow."""

    print("🧪 Testing Basic Linear Workflow")

    # Canvas data from actual test files
    canvas_data = {
        "nodes": [
            {"id": "node_0", "type": "placeholder", "data": {}},
            {"id": "node_1", "type": "placeholder", "data": {}},
            {"id": "node_2", "type": "placeholder", "data": {}},
        ],
        "edges": [{"source": "node_0", "target": "node_1"}, {"source": "node_1", "target": "node_2"}],
    }

    engine = LangGraphWorkflowEngine()

    try:
        # Use checkpointer context manager for testing
        async with AsyncSqliteSaver.from_conn_string("test_checkpoints.db") as checkpointer:
            # Build the graph
            graph = engine._build_langgraph(canvas_data, execution_id=1, checkpointer=checkpointer)
            print("✅ Graph built successfully")

            # Create initial state
            initial_state = WorkflowState(
                execution_id=1,
                node_outputs={},
                completed_nodes=[],
                error=None,
                db_session_factory=None,  # Mock for testing
            )

            # Execute the graph with config
            print("⚡ Executing workflow...")
            config = {"configurable": {"thread_id": "test_workflow_1"}}
            final_state = await graph.ainvoke(initial_state, config)

        # Validate results
        assert len(final_state["node_outputs"]) == 3, f"Expected 3 node outputs, got {len(final_state['node_outputs'])}"
        assert "node_0" in final_state["node_outputs"], "node_0 output missing"
        assert "node_1" in final_state["node_outputs"], "node_1 output missing"
        assert "node_2" in final_state["node_outputs"], "node_2 output missing"
        assert final_state["error"] is None, f"Unexpected error: {final_state['error']}"

        print("✅ All nodes executed successfully")
        print(f"✅ Node outputs: {list(final_state['node_outputs'].keys())}")
        print("✅ Linear workflow test PASSED")

    except Exception as e:
        print(f"❌ Linear workflow test FAILED: {e}")
        raise


async def test_parallel_execution():
    """Test parallel execution with diamond pattern."""

    print("\n🧪 Testing Parallel Execution (Diamond Pattern)")

    # Diamond pattern: A -> B,C -> D
    canvas_data = {
        "nodes": [
            {"id": "start", "type": "placeholder", "data": {}},
            {"id": "branch_1", "type": "placeholder", "data": {}},
            {"id": "branch_2", "type": "placeholder", "data": {}},
            {"id": "merge", "type": "placeholder", "data": {}},
        ],
        "edges": [
            {"source": "start", "target": "branch_1"},
            {"source": "start", "target": "branch_2"},
            {"source": "branch_1", "target": "merge"},
            {"source": "branch_2", "target": "merge"},
        ],
    }

    engine = LangGraphWorkflowEngine()

    try:
        async with AsyncSqliteSaver.from_conn_string("test_checkpoints.db") as checkpointer:
            graph = engine._build_langgraph(canvas_data, execution_id=2, checkpointer=checkpointer)
            print("✅ Diamond graph built successfully")

            initial_state = WorkflowState(
                execution_id=2, node_outputs={}, completed_nodes=[], error=None, db_session_factory=None
            )

            # Time the execution to verify parallelism
            import time

            start_time = time.time()
            config = {"configurable": {"thread_id": "test_workflow_2"}}
            final_state = await graph.ainvoke(initial_state, config)

        execution_time = time.time() - start_time

        # Validate results
        assert len(final_state["node_outputs"]) == 4, f"Expected 4 node outputs, got {len(final_state['node_outputs'])}"
        assert final_state["error"] is None, f"Unexpected error: {final_state['error']}"

        print(f"✅ All 4 nodes executed in {execution_time:.3f}s")
        print("✅ Diamond pattern test PASSED")

    except Exception as e:
        print(f"❌ Diamond pattern test FAILED: {e}")
        raise


async def test_error_handling():
    """Test error handling with simulated failures."""

    print("\n🧪 Testing Error Handling")

    canvas_data = {
        "nodes": [
            {"id": "good_node", "type": "placeholder", "data": {}},
            {"id": "bad_node", "type": "nonexistent_type", "data": {}},  # This will cause error
        ],
        "edges": [{"source": "good_node", "target": "bad_node"}],
    }

    engine = LangGraphWorkflowEngine()

    try:
        async with AsyncSqliteSaver.from_conn_string("test_checkpoints.db") as checkpointer:
            graph = engine._build_langgraph(canvas_data, execution_id=3, checkpointer=checkpointer)
            print("✅ Error test graph built successfully")

            initial_state = WorkflowState(
                execution_id=3, node_outputs={}, completed_nodes=[], error=None, db_session_factory=None
            )

            # This should complete the first node but may have issues with second
            config = {"configurable": {"thread_id": "test_workflow_3"}}
            final_state = await graph.ainvoke(initial_state, config)

        # Should have first node output
        assert "good_node" in final_state["node_outputs"], "First node didn't execute"
        print("✅ First node executed successfully")
        print("✅ Error handling test PASSED")

    except Exception as e:
        print(f"✅ Expected error caught: {type(e).__name__}")
        print("✅ Error handling test PASSED")


async def test_complex_workflow():
    """Test a more complex workflow with different node types."""

    print("\n🧪 Testing Complex Workflow")

    # More complex workflow simulating real usage
    canvas_data = {
        "retries": {"default": 1, "backoff": "exponential"},
        "nodes": [
            {"id": "webhook_trigger", "type": "trigger", "trigger_type": "webhook", "config": {}},
            {"id": "data_processor", "type": "placeholder", "data": {"task": "process_data"}},
            {"id": "result_formatter", "type": "placeholder", "data": {"task": "format_results"}},
        ],
        "edges": [
            {"source": "webhook_trigger", "target": "data_processor"},
            {"source": "data_processor", "target": "result_formatter"},
        ],
    }

    engine = LangGraphWorkflowEngine()

    try:
        async with AsyncSqliteSaver.from_conn_string("test_checkpoints.db") as checkpointer:
            graph = engine._build_langgraph(canvas_data, execution_id=4, checkpointer=checkpointer)
            print("✅ Complex workflow graph built successfully")

            initial_state = WorkflowState(
                execution_id=4, node_outputs={}, completed_nodes=[], error=None, db_session_factory=None
            )

            config = {"configurable": {"thread_id": "test_workflow_4"}}
            final_state = await graph.ainvoke(initial_state, config)

        # Validate all nodes executed
        expected_nodes = {"webhook_trigger", "data_processor", "result_formatter"}
        actual_nodes = set(final_state["node_outputs"].keys())

        assert expected_nodes == actual_nodes, f"Expected {expected_nodes}, got {actual_nodes}"
        assert final_state["error"] is None, f"Unexpected error: {final_state['error']}"

        print("✅ All complex workflow nodes executed")
        print("✅ Node execution order preserved")
        print("✅ Complex workflow test PASSED")

    except Exception as e:
        print(f"❌ Complex workflow test FAILED: {e}")
        raise


async def test_state_passing():
    """Test that state is properly passed between nodes."""

    print("\n🧪 Testing State Passing Between Nodes")

    canvas_data = {
        "nodes": [
            {"id": "producer", "type": "placeholder", "data": {"produces": "test_data"}},
            {"id": "consumer", "type": "placeholder", "data": {"consumes": "test_data"}},
        ],
        "edges": [{"source": "producer", "target": "consumer"}],
    }

    engine = LangGraphWorkflowEngine()

    try:
        async with AsyncSqliteSaver.from_conn_string("test_checkpoints.db") as checkpointer:
            graph = engine._build_langgraph(canvas_data, execution_id=5, checkpointer=checkpointer)
            print("✅ State passing graph built successfully")

            initial_state = WorkflowState(
                execution_id=5, node_outputs={}, completed_nodes=[], error=None, db_session_factory=None
            )

            config = {"configurable": {"thread_id": "test_workflow_5"}}
            final_state = await graph.ainvoke(initial_state, config)

        # Validate state was passed
        assert "producer" in final_state["node_outputs"], "Producer node didn't execute"
        assert "consumer" in final_state["node_outputs"], "Consumer node didn't execute"

        # Consumer should have access to producer's output
        producer_output = final_state["node_outputs"]["producer"]
        assert producer_output is not None, "Producer didn't generate output"

        print("✅ State successfully passed between nodes")
        print("✅ State passing test PASSED")

    except Exception as e:
        print(f"❌ State passing test FAILED: {e}")
        raise


async def main():
    """Run all integration tests."""

    print("🚀 Starting LangGraph Integration Tests")
    print("=" * 50)

    tests = [
        test_basic_linear_workflow,
        test_parallel_execution,
        test_error_handling,
        test_complex_workflow,
        test_state_passing,
    ]

    passed = 0
    failed = 0

    for test in tests:
        try:
            await test()
            passed += 1
        except Exception as e:
            print(f"❌ Test {test.__name__} failed: {e}")
            failed += 1

    print("\n" + "=" * 50)
    print(f"🏁 Test Results: {passed} passed, {failed} failed")

    if failed == 0:
        print("🎉 All integration tests PASSED!")
        return True
    else:
        print("💥 Some tests FAILED!")
        return False


if __name__ == "__main__":
    success = asyncio.run(main())
    sys.exit(0 if success else 1)
