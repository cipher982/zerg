#!/bin/bash
set -e

echo "Building frontend (debug, no server)…"

# Confirm wasm-pack exists
if ! command -v wasm-pack >/dev/null 2>&1; then
  echo "Error: wasm-pack not found.  Install with 'cargo install wasm-pack'." >&2
  exit 1
fi

# Build WASM package
RUSTFLAGS="-C debuginfo=2" wasm-pack build --dev --target web --out-dir www

# Bootstrap loader identical to dev script (minus server)
echo "[build-only] writing bootstrap.js …"
cat > www/bootstrap.js 
import init, { init_api_config_js } from './agent_platform_frontend.js';

async function main() {
  await init();
  init_api_config_js('http://localhost:8001');
}

main();
EOF

# Stub config.js if missing
echo "[build-only] ensuring config.js …"
if [ ! -f www/config.js ]; then
  cat <<'EOF' > www/config.js
window.__APP_CONFIG__ = window.__APP_CONFIG__ || {};
window.__APP_CONFIG__.BUILD = 'debug';
EOF
fi

echo "Build complete – artefacts in ./www/"