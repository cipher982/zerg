#!/bin/bash
set -e

echo "🔄 Running database migrations..."
python -m alembic upgrade head

echo "✅ Migrations complete, starting server..."
exec python -m uvicorn zerg.main:app --host 0.0.0.0 --port 8000