# WASM Draggable Box Demo

A simple WebAssembly demo that creates a draggable box using Rust and wasm-bindgen.

## Prerequisites

- [Rust](https://www.rust-lang.org/tools/install)
- [wasm-pack](https://rustwasm.github.io/wasm-pack/installer/)

## Building the Demo

1. Make sure you have Rust and wasm-pack installed
2. Run the build script:

```bash
./build.sh
```

## Running the Demo

After building, you can open `www/index.html` in a web browser to see the demo.
For the best experience, serve the files using a local development server:

```bash
# Using Python 3
python -m http.server -d www

# Or using Python 2
python -m SimpleHTTPServer
```

Then visit `http://localhost:8000` in your browser.

## What's Happening?

1. Rust code is compiled to WebAssembly via wasm-pack
2. The WASM module creates a blue box on the page
3. Mouse event handlers allow you to drag the box around

This demonstrates how WASM can be used for interactive UI elements with near-native performance! 