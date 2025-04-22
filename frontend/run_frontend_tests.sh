#!/bin/bash

# Set the ChromeDriver path
export CHROMEDRIVER="$HOME/bin/chromedriver"

# Configure test runner environment
export CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER="$HOME/Library/Caches/.wasm-pack/wasm-bindgen-cargo-install-0.2.100/wasm-bindgen-test-runner"
export WASM_BINDGEN_TEST_ONLY_WEB="1"

# Run the tests
cargo test --target wasm32-unknown-unknown "$@"