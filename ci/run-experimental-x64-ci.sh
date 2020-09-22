#!/bin/bash

# Use the Nightly variant of the compiler to properly unify the
# experimental_x64 feature across all crates.  Once the feature has stabilized
# and become the default, we can remove this.
CARGO_VERSION=${CARGO_VERSION:-"+nightly"}

cargo $CARGO_VERSION \
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
