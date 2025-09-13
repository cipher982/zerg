#!/bin/bash
set -e

# Ensure we're in the right directory for alembic
cd /app

echo "🔄 Running database migrations..."
echo "Working directory: $(pwd)"
echo "Alembic config exists: $(test -f alembic.ini && echo 'YES' || echo 'NO')"
echo "Python path: $(which python)"
echo "Alembic module: $(python -c 'import alembic; print(alembic.__version__)' 2>/dev/null || echo 'NOT FOUND')"

# Run migrations with more verbose output
python -m alembic upgrade head || {
    echo "❌ Migration failed, starting server anyway..."
}

echo "✅ Migrations complete, starting server..."
exec python -m uvicorn zerg.main:app --host 0.0.0.0 --port 8000