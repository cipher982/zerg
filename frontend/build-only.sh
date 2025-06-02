#!/bin/bash
set -e

echo "Building frontend (debug, no server)…"

# Confirm wasm-pack exists
if ! command -v wasm-pack >/dev/null 2>&1; then
  echo "Error: wasm-pack not found.  Install with 'cargo install wasm-pack'." >&2
  exit 1
fi

# Load repo-root .env so variables like API_BASE_URL can be overridden
ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
ENV_FILE="$ROOT_DIR/.env"
if [ -f "$ENV_FILE" ]; then
  set -a
  # shellcheck source=/dev/null
  . "$ENV_FILE"
  set +a
fi

# Default if not provided via env
API_BASE_URL="${API_BASE_URL:-http://localhost:8001}"

echo "Building WASM module..."
API_BASE_URL="$API_BASE_URL" RUSTFLAGS="-C debuginfo=2" wasm-pack build --dev --target web --out-dir www

# Bootstrap loader identical to dev script (minus server)
echo "[build-only] writing bootstrap.js …"
cat > www/bootstrap.js 
import init, { init_api_config_js } from './agent_platform_frontend.js';

# Create config.js + bootstrap.js so tests and manual runs find them
cat > www/config.js <<'EOF'
// Auto-generated – build-only.sh
window.ZERG_CONFIG = {
  API_BASE_URL: window.API_BASE_URL || 'http://localhost:8001',
};
EOF

cat > www/bootstrap.js <<'EOF'
import './index.js';
EOF

echo "Frontend build complete. Files are ready in www/ directory."
