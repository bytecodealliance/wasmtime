#!/bin/bash

cargo test \
    --features "test-programs/test_programs" \
    --features wasi-threads \
    --workspace \
    --exclude 'wasmtime-wasi-*' \
    --exclude wasi-crypto \
    $@
