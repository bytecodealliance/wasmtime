#!/bin/bash

EXTRA_FEATURE=""

if [[ $1 != "" ]]
then
    EXTRA_FEATURE="--features $1"
fi

cargo test \
    --features "test-programs/test_programs wiggle/wasmtime_async wasmtime/wat" \
    $EXTRA_FEATURE \
    --locked \
    --workspace \
    --exclude '*lightbeam*' \
    --exclude 'wasmtime-wasi-*' \
    --exclude 'peepmatic*' \
    --exclude wasi-crypto
