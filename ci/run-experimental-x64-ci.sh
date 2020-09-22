#!/bin/bash

cargo +nightly \
            -Zfeatures=all -Zpackage-features \
            test \
            --features test-programs/test_programs \
            --features experimental_x64 \
            --all \
            --exclude wasmtime-lightbeam \
            --exclude peepmatic \
            --exclude peepmatic-automata \
            --exclude peepmatic-fuzzing \
            --exclude peepmatic-macro \
            --exclude peepmatic-runtime \
            --exclude peepmatic-test \
            --exclude peepmatic-souper \
            --exclude lightbeam \
            $@
