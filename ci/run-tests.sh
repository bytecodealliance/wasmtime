#!/bin/bash

cargo test \
    --features wasi-threads \
    --features wasi-http \
    --features component-model \
    --features serve \
    --workspace \
    --exclude test-programs \
    $@
