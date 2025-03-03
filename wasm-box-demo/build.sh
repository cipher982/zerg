#!/bin/bash
set -e

# Check if wasm-pack is installed
if ! command -v wasm-pack &> /dev/null; then
    echo "wasm-pack is not installed. Installing now..."
    cargo install wasm-pack
fi

# Build the WASM module
echo "Building WASM module..."
wasm-pack build --target web --out-dir www

# Move the generated JS to index.js for simplicity
echo "Finalizing build..."
mv www/wasm_box_demo.js www/index.js
sed -i '' 's/wasm_box_demo_bg.wasm/wasm_box_demo_bg.wasm/g' www/index.js

echo "Build complete! Open www/index.html in a browser to see the demo." 