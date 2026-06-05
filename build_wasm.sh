#!/bin/bash
# Enable cargo and wasm-pack in the PATH
export PATH="/home/jason/.cargo/bin:$PATH"

echo "=== Environment Info ==="
echo "PATH: $PATH"
echo "Cargo: $(which cargo)"
cargo --version
echo "wasm-pack: $(which wasm-pack)"
wasm-pack --version

echo "=== Building WASM target: web ==="
wasm-pack build --target web
echo "=== Build Complete ==="
