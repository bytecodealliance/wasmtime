#!/bin/bash

cargo test \
    --features wasi-threads \
    --features wasi-http \
    --features component-model \
    --features serve \
    --features wasmtime-wasi-nn/onnx \
    --workspace \
    --exclude test-programs \
    $@
