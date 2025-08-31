#!/bin/bash
# Clean up worker-specific test database files

cd "$(dirname "$0")"
python cleanup_test_dbs.py
