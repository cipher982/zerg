#!/bin/bash
set -e

# Ensure we're in the right directory for alembic
cd /app

# Create migration log file accessible via web
MIGRATION_LOG="/app/static/migration.log"
mkdir -p /app/static

echo "=== Migration Log $(date) ===" > "$MIGRATION_LOG"

{
    echo "🔄 Running database migrations..."
    echo "Working directory: $(pwd)"
    echo "Alembic config exists: $(test -f alembic.ini && echo 'YES' || echo 'NO')"
    echo "Python path: $(which python)"
    echo "Alembic module check:"
    python -c 'import alembic; print(f"Alembic version: {alembic.__version__}")' 2>&1 || echo 'ALEMBIC NOT FOUND'
    echo "Database URL check:"
    python -c 'from zerg.config import get_settings; s=get_settings(); print(f"DB URL: {s.database_url[:30]}...")' 2>&1 || echo 'CONFIG ERROR'
    
    echo "Running alembic upgrade head..."
    python -m alembic upgrade head 2>&1 || {
        echo "❌ Migration failed with exit code $?"
        echo "Migration FAILED - server starting anyway"
    }
    
    echo "✅ Migration process complete"
} | tee -a "$MIGRATION_LOG"

echo "Starting server..."
exec python -m uvicorn zerg.main:app --host 0.0.0.0 --port 8000