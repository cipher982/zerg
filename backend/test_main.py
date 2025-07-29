"""Test application entry point.

This module provides the entry point for the test version of the application,
used by E2E tests with isolated worker databases.
"""

import os

# Must set environment before any zerg imports
if "ENVIRONMENT" not in os.environ:
    os.environ["ENVIRONMENT"] = "test"

from zerg.core.factory import create_app  # noqa: E402

# Create app with test configuration
app = create_app()

if __name__ == "__main__":
    import uvicorn

    worker_id = os.getenv("TEST_WORKER_ID", "0")
    port = 8000 + int(worker_id)

    uvicorn.run(app, host="0.0.0.0", port=port)
