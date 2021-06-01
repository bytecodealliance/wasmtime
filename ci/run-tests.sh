#!/bin/bash

cargo test \
    --features "test-programs/test_programs" \
    --workspace \
    --exclude '*lightbeam*' \
    --exclude 'wasmtime-wasi-*' \
    --exclude 'peepmatic*' \
    --exclude wasi-crypto \
    $@
