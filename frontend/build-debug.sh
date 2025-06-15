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

# Default API_BASE_URL if not provided via .env
API_BASE_URL="${API_BASE_URL:-http://localhost:8001}"

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
cat > www/bootstrap.js <<'EOF'
import init, { init_api_config_js } from './agent_platform_frontend.js';

async function main() {
  await init();            // loads wasm & runs #[wasm_bindgen(start)]
  init_api_config_js('http://localhost:8001');
}

main();
EOF

# Stub config.js so the HTML include does not 404 in dev.
echo "[build-debug] ensuring config.js …"
if [ ! -f www/config.js ]; then
  cat <<'EOF' > www/config.js
window.__APP_CONFIG__ = window.__APP_CONFIG__ || {};
window.__APP_CONFIG__.BUILD = 'debug';
EOF
fi

echo "[build-debug] starting dev server on http://localhost:8002 …"
cd www && python3 -m http.server 8002
