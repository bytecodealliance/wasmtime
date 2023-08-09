#!/bin/bash

cargo test \
    --features "test-programs/test_programs" \
    --features wasi-threads \
    --features wasi-http \
    --workspace \
    --exclude 'wasmtime-wasi-*' \
    --exclude wasi-tests \
    --exclude command-tests \
    --exclude reactor-tests \
    $@
