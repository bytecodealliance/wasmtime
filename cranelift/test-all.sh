#!/bin/bash
set -euo pipefail

# This is the top-level test script:
#
# - Make a debug build.
# - Make a release build.
# - Run unit tests for all Rust crates (including the filetests)
# - Build API documentation.
#
# All tests run by this script should be passing at all times.

# Disable generation of .pyc files because they cause trouble for vendoring
# scripts, and this is a build step that isn't run very often anyway.
export PYTHONDONTWRITEBYTECODE=1

# Repository top-level directory.
cd $(dirname "$0")
topdir=$(pwd)

function banner {
    echo "======  $@  ======"
}

# Run rustfmt if we have it.
banner "Rust formatting"
if command -v rustfmt > /dev/null; then
    # In newer versions of rustfmt, replace --write-mode=diff with --check.
    if ! $topdir/format-all.sh --write-mode=diff ; then
        echo "Formatting diffs detected! Run \"cargo fmt --all\" to correct."
        exit 1
    fi
else
    echo "rustfmt not available; formatting not checked!"
    echo
    echo "If you are using rustup, rustfmt can be installed via"
    echo "\"rustup component add --toolchain=stable rustfmt-preview\", or see"
    echo "https://github.com/rust-lang-nursery/rustfmt for more information."
fi

# Check if any Python files have changed since we last checked them.
tsfile=$topdir/target/meta-checked
if [ -f $tsfile ]; then
    needcheck=$(find $topdir/lib/codegen/meta -name '*.py' -newer $tsfile)
else
    needcheck=yes
fi
if [ -n "$needcheck" ]; then
    banner "$(python --version 2>&1), $(python3 --version 2>&1)"
    $topdir/lib/codegen/meta/check.sh
    touch $tsfile || echo no target directory
fi

# Make sure the code builds in release mode.
banner "Rust release build"
cargo build --release

# Make sure the code builds in debug mode.
banner "Rust debug build"
cargo build

# Run the tests. We run these in debug mode so that assertions are enabled.
banner "Rust unit tests"
cargo test --all

# Make sure the documentation builds.
banner "Rust documentation: $topdir/target/doc/cretonne/index.html"
cargo doc

# Run clippy if we have it.
banner "Rust linter"
if $topdir/check-clippy.sh; then
    $topdir/clippy-all.sh --write-mode=diff
else
    echo "\`cargo +nightly install clippy\` for optional rust linting"
fi

# Ensure fuzzer works by running it with a single input
# Note LSAN is disabled due to https://github.com/google/sanitizers/issues/764
banner "cargo fuzz check"
if rustup toolchain list | grep -q nightly; then
    if cargo install --list | grep -q cargo-fuzz; then
        echo "cargo-fuzz found"
    else
        echo "installing cargo-fuzz"
        cargo +nightly install cargo-fuzz
    fi
    ASAN_OPTIONS=detect_leaks=0 cargo +nightly fuzz run fuzz_translate_module $topdir/fuzz/corpus/fuzz_translate_module/ffaefab69523eb11935a9b420d58826c8ea65c4c
else
    echo "nightly toolchain not found, skipping fuzz target integration test"
fi

banner "OK"
