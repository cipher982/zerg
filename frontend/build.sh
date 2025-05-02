#!/bin/bash
set -e

# Check if wasm-pack is installed
if ! command -v wasm-pack &> /dev/null; then
    echo "wasm-pack is not installed. Please install it first:"
    echo "cargo install wasm-pack"
    exit 1
fi

# Build the WASM module
echo "Building WASM module..."
# Ensure GOOGLE_CLIENT_ID is set for compile-time embedding
if [[ -z "${GOOGLE_CLIENT_ID}" ]]; then
  echo "WARNING: GOOGLE_CLIENT_ID not set, frontend will fall back to empty client_id."
fi

# Expose the variable to Cargo so `env!("GOOGLE_CLIENT_ID")` works.
GOOGLE_CLIENT_ID="${GOOGLE_CLIENT_ID}" wasm-pack build --target web --out-dir www

# Move the generated JS to index.js for simplicity
echo "Finalizing build..."
mv www/agent_platform_frontend.js www/index.js
sed -i '' 's/agent_platform_frontend_bg.wasm/agent_platform_frontend_bg.wasm/g' www/index.js

cd www
python -m http.server 8002