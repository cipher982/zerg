#!/bin/bash
set -e

# Load environment overrides if present
if [ -f .env ]; then
  # shellcheck disable=SC2046
  export $(grep -v '^#' .env | xargs)
fi

echo "[build-debug] HELLO compiling WASM (debug)..."
RUSTFLAGS="-C debuginfo=2" wasm-pack build --dev --target web --out-dir www

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