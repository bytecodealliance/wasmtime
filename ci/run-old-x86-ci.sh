#!/bin/bash

cargo test \
            --locked \
            --features test-programs/test_programs \
            --features old-x86-backend \
            --all \
            --exclude wasmtime-lightbeam \
            --exclude wasmtime-wasi-nn \
            --exclude wasmtime-wasi-crypto \
            --exclude peepmatic \
            --exclude peepmatic-automata \
            --exclude peepmatic-fuzzing \
            --exclude peepmatic-macro \
            --exclude peepmatic-runtime \
            --exclude peepmatic-test \
            --exclude peepmatic-souper \
            --exclude lightbeam
