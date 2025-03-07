#!/bin/bash

# Build with debug profile and source maps
RUSTFLAGS="-C debuginfo=2" wasm-pack build --dev --target web --out-dir www

# Copy the generated JavaScript file to index.js for simplicity
cp www/agent_platform_frontend.js www/index.js

# Start a Python HTTP server on port 8002 (or use your preferred server)
cd www && python3 -m http.server 8002 