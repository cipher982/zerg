#!/bin/bash
set -e

# E2E test runner: launches backend with TESTING=1 and runs Playwright tests against it

# Kill any previous test backend
if lsof -i:8002 >/dev/null 2>&1; then
  echo "Killing previous backend on port 8002"
  lsof -ti:8002 | xargs kill -9
fi

# Start backend with TESTING=1 in the background
echo "Starting backend with TESTING=1 on port 8002"
(
  cd ../backend
  TESTING=1 LOG_LEVEL=WARNING uv run python -m uvicorn zerg.main:app --host 0.0.0.0 --port 8002
) &
BACKEND_PID=$!

# Wait for backend to be ready
echo "Waiting for backend to start..."
for i in {1..30}; do
  if curl -s http://localhost:8002/ | grep -q 'Agent Platform API'; then
    echo "Backend is up!"
    break
  fi
  sleep 1
done

# Run Playwright E2E tests
echo "Running Playwright E2E tests..."
npm test

# Kill backend after tests
echo "Stopping backend..."
kill $BACKEND_PID
wait $BACKEND_PID 2>/dev/null || true

echo "E2E tests complete."
