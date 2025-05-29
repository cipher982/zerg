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

# Build with debug profile and set API_BASE_URL for e2e tests
echo "Building WASM module..."
API_BASE_URL="http://localhost:8001" RUSTFLAGS="-C debuginfo=2" wasm-pack build --dev --target web --out-dir www

# Copy the generated JavaScript file to index.js for simplicity
echo "Copying JS files..."
cp www/agent_platform_frontend.js www/index.js

echo "Frontend build complete. Files are ready in www/ directory."
