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
wasm-pack build --target web --out-dir www

# Move the generated JS to index.js for simplicity
echo "Finalizing build..."
mv www/agent_platform_frontend.js www/index.js
sed -i '' 's/agent_platform_frontend_bg.wasm/agent_platform_frontend_bg.wasm/g' www/index.js

echo "Build complete! To run the frontend:"
echo "1. Start the Python backend: cd ../backend && python main.py"
echo "2. In another terminal, serve the frontend: cd www && python -m http.server"
echo "3. Open http://localhost:8000 in your browser" 