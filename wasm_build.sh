#! /bin/bash

if [[ "$1" == "--dev" ]]; then
  RELEASE_PROFILE="--dev"
else
  RELEASE_PROFILE="--release"
fi
# Build the WebAssembly package into ./pkg (loaded by index.js).
#   --target web   : output a plain ES module the browser can import, no bundler needed.
#   RUSTFLAGS      : tells the `getrandom` crate (used by `rand`) to get randomness from the browser.
# Pass --dev for a faster, unoptimised build with debug info.
RUSTFLAGS='--cfg getrandom_backend="wasm_js"' wasm-pack build --target web "$RELEASE_PROFILE"
