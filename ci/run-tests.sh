#!/bin/bash

cargo test \
    --features wasi-threads \
    --features wasi-http \
    --features component-model \
    --features serve \
    --features wasmtime-wasi-nn/test-check \
    --workspace \
    --exclude test-programs \
    $@
