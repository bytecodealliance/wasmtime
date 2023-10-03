#!/bin/bash

cargo test \
    --features "test-programs/test_programs" \
    --features wasi-threads \
    --features wasi-http \
    --features component-model \
    --features serve \
    --workspace \
    --exclude 'wasmtime-wasi-*' \
    --exclude wasi-tests \
    --exclude wasi-http-tests \
    --exclude command-tests \
    --exclude reactor-tests \
    $@
