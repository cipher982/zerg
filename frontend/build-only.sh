#!/bin/bash
set -e

# Build the frontend without starting a server
echo "Building frontend for e2e tests..."

# Check if wasm-pack is installed
if ! command -v wasm-pack &> /dev/null; then
    echo "wasm-pack is not installed. Please install it first:"
    echo "cargo install wasm-pack"
    exit 1
fi

# Build with debug profile
echo "Building WASM module..."
RUSTFLAGS="-C debuginfo=2" wasm-pack build --dev --target web --out-dir www

# Copy the generated JavaScript file to index.js for simplicity
echo "Copying JS files..."
cp www/agent_platform_frontend.js www/index.js

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
