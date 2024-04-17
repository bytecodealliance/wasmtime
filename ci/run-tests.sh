#!/bin/bash

set -euo pipefail

cargo_flags=(
    --features wasmtime-cli/wasi-threads
    --features wasmtime-cli/wasi-http
    --features wasmtime-cli/component-model
    --features wasmtime-cli/serve
    --workspace
    --exclude test-programs
)

if [ "${USE_NEXTEST:-0}" = "1" ]; then
  cargo nextest run "${cargo_flags[@]}" "$@"
else
  cargo test "${cargo_flags[@]}" "$@"
fi
