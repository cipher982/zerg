#!/bin/bash
# Clean up worker-specific test database files

cd "$(dirname "$0")"
if ! command -v python3 >/dev/null 2>&1; then
  echo "ERROR: python3 is required to run backend/cleanup_test_dbs.sh" >&2
  echo "Install Python 3 or set PYTHON to a valid python3 path and rerun." >&2
  exit 1
fi

python3 cleanup_test_dbs.py
