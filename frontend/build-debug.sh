#!/bin/bash
set -e

# Load *repository-root* .env so frontend uses the same configuration as backend
ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
ENV_FILE="$ROOT_DIR/.env"

if [ -f "$ENV_FILE" ]; then
  # Enable automatic export of all assigned variables
  set -a
  # shellcheck source=/dev/null
  . "$ENV_FILE"
  set +a
fi

echo "[build-debug] HELLO compiling WASM (debug)..."

# Ensure a writable TMPDIR – some sandboxed CI runners mount the default
# /var/folders/... macOS location read-only which breaks Cargo during
# compilation (it needs to create temporary crates & metadata files).  We
# create a local directory inside the repository as a safe fallback.

if [[ -z "${TMPDIR:-}" || ! -w "${TMPDIR}" ]]; then
  export TMPDIR="$(pwd)/.tmp_build"
  mkdir -p "$TMPDIR"
fi

# Configure ports from .env with fallback defaults  
BACKEND_PORT="${BACKEND_PORT:-8001}"
FRONTEND_PORT="${FRONTEND_PORT:-8002}"
API_BASE_URL="${API_BASE_URL:-http://localhost:${BACKEND_PORT}}"

# -------------------------------------------------------------
# RUSTFLAGS — preserve existing debuginfo flag *and* opt-in to
# the getrandom JS backend required for wasm32-unknown-unknown.
# When RUSTFLAGS is set directly (instead of via .cargo/config.toml)
# it overrides the project-level configuration, so we must include
# the `--cfg getrandom_backend="wasm_js"` flag here as well.
# -------------------------------------------------------------

RUSTFLAGS="--cfg getrandom_backend=\"wasm_js\" -C debuginfo=2" \
  API_BASE_URL="$API_BASE_URL" \
  wasm-pack build --dev --target web --out-dir pkg

# Copy the generated files to www directory
echo "[build-debug] copying WASM artifacts to www..."
cp pkg/agent_platform_frontend.js www/
cp pkg/agent_platform_frontend_bg.wasm www/
cp pkg/agent_platform_frontend.d.ts www/
cp pkg/agent_platform_frontend_bg.wasm.d.ts www/

# -------------------------------------------------------------
# Generate bootstrap.js expected by index.html
# -------------------------------------------------------------
echo "[build-debug] writing bootstrap.js …"
cat > www/bootstrap.js <<EOF
import init, { init_api_config_js } from './agent_platform_frontend.js';

async function main() {
  await init();            // loads wasm & runs #[wasm_bindgen(start)]
  const url = window.API_BASE_URL || 'http://localhost:${BACKEND_PORT}';
  init_api_config_js(url);
}

main();
EOF

# Generate index.html from template with dynamic values
echo "[build-debug] generating index.html from template for ports ${BACKEND_PORT}/${FRONTEND_PORT} …"
TIMESTAMP=$(date +%s)
CACHE_BUST_TAG="<meta name=\"cache-bust\" content=\"${TIMESTAMP}\">"

# Generate index.html from template
sed \
  -e "s|{{BACKEND_PORT}}|${BACKEND_PORT}|g" \
  -e "s|{{CACHE_BUST}}|${CACHE_BUST_TAG}|g" \
  www/index.html.template > www/index.html

# Stub config.js so the HTML include does not 404 in dev.
echo "[build-debug] ensuring config.js …"
if [ ! -f www/config.js ]; then
  cat <<'EOF' > www/config.js
window.__APP_CONFIG__ = window.__APP_CONFIG__ || {};
window.__APP_CONFIG__.BUILD = 'debug';
EOF
fi

echo "[build-debug] starting dev server on http://localhost:${FRONTEND_PORT} …"
cd www

# Create a simple HTTP server with cache control headers
cat > server.py << 'EOF'
import http.server
import socketserver
import sys

class CacheControlHandler(http.server.SimpleHTTPRequestHandler):
    def end_headers(self):
        # Disable caching for HTML files to ensure CSP updates are loaded
        if self.path.endswith('.html') or self.path == '/':
            self.send_header('Cache-Control', 'no-cache, no-store, must-revalidate')
            self.send_header('Pragma', 'no-cache')
            self.send_header('Expires', '0')
        super().end_headers()

PORT = int(sys.argv[1]) if len(sys.argv) > 1 else 8000
with socketserver.TCPServer(("", PORT), CacheControlHandler) as httpd:
    print(f"Serving at http://localhost:{PORT}")
    httpd.serve_forever()
EOF

python3 server.py ${FRONTEND_PORT}
