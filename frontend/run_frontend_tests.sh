#!/bin/bash

# Set the ChromeDriver path
export CHROMEDRIVER="$HOME/bin/chromedriver"

# Configure test runner environment
export CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER="$HOME/Library/Caches/.wasm-pack/wasm-bindgen-cargo-install-0.2.100/wasm-bindgen-test-runner"
export WASM_BINDGEN_TEST_ONLY_WEB="1"

# Workaround for macOS sandbox which sometimes blocks Rust from creating
# temporary directories under the default $TMPDIR.  Falling back to a local
# folder fixes "couldn't create a temp dir: Operation not permitted" errors
# that break CI when compiling `wasm-bindgen-test`.
export TMPDIR="$(pwd)/.tmp-tests"
mkdir -p "$TMPDIR"

# Run the tests
cargo test --target wasm32-unknown-unknown "$@"