#!/bin/bash

cargo test \
    --features "test-programs/test_programs" \
    --workspace \
    --exclude 'wasmtime-wasi-*' \
    --exclude wasi-crypto \
    $@
