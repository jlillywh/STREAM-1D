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
echo "=== Building WASM target: nodejs ==="
wasm-pack build --target nodejs --out-dir pkg-node
echo "=== JSON contract tests ==="
cargo test --test wasm_json_contract --quiet
echo "=== Node WASM smoke tests ==="
node examples/wasm/node_smoke_test.mjs
node examples/wasm/bridge_smoke_test.mjs
echo "=== Build Complete ==="
