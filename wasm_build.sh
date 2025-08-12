#! /bin/bash

if [[ "$1" == "--dev" ]]; then
  RELEASE_PROFILE="--dev"
else
  RELEASE_PROFILE="--release"
fi
RUSTFLAGS='--cfg getrandom_backend="wasm_js"' wasm-pack build --target web "$RELEASE_PROFILE"
