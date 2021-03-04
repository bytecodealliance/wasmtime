#!/bin/bash

# Use the Nightly variant of the compiler to properly unify the
# experimental_x64 feature across all crates.  Once the feature has stabilized
# and become the default, we can remove this.
CARGO_VERSION=${CARGO_VERSION:-"+nightly"}

# Some WASI tests seem to have an issue on Windows with symlinks if we run them
# with this particular invocation. It's unclear why (nightly toolchain?) but
# we're moving to the new backend by default soon enough, and all tests seem to
# work with the main test setup, so let's just work around this by skipping
# the tests for now.
MINGW_EXTRA=""
if [ `uname -o` == "Msys" ]; then
	MINGW_EXTRA="-- --skip wasi_cap_std_sync"
fi

cargo $CARGO_VERSION \
            --locked \
            -Zfeatures=all -Zpackage-features \
            test \
            --features test-programs/test_programs \
            --features experimental_x64 \
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
            --exclude lightbeam \
	    $MINGW_EXTRA
