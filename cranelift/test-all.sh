#!/bin/bash
set -euo pipefail

# This is the top-level test script:
#
# - Check code formatting.
# - Make a debug build.
# - Make a release build.
# - Run unit tests for all Rust crates (including the filetests)
# - Build API documentation.
# - Optionally, run fuzzing.
#
# All tests run by this script should be passing at all times.

# Repository top-level directory.
topdir=$(dirname "$0")
cd "$topdir"

function banner {
    echo "======  $*  ======"
}

# Run rustfmt if we have it.
banner "Rust formatting"
if cargo +stable fmt -- --version > /dev/null ; then
    if ! "$topdir/format-all.sh" --check ; then
        echo "Formatting diffs detected! Run \"cargo fmt --all\" to correct."
        exit 1
    fi
else
    echo "cargo-fmt not available; formatting not checked!"
    echo
    echo "If you are using rustup, rustfmt can be installed via"
    echo "\"rustup component add --toolchain=stable rustfmt-preview\", or see"
    echo "https://github.com/rust-lang-nursery/rustfmt for more information."
fi

# Make sure the code builds in release mode.
banner "Rust release build"
cargo build --release

# Make sure the code builds in debug mode.
banner "Rust debug build"
cargo build

# Run the tests. We run these in debug mode so that assertions are enabled.
banner "Rust unit tests"
RUST_BACKTRACE=1 cargo test --all

has_toolchain() {
    rustup toolchain list | grep -q $1
}

ensure_installed() {
    program="$1"
    toolchain="${2:-stable}"
    if has_toolchain $toolchain; then
        if grep -q $program <(cargo +$toolchain install --list); then
            echo "$program found"
        else
            echo "installing $program"
            cargo +$toolchain install $program
        fi
    else
        return 1
    fi
}

# Make sure the documentation builds.
banner "Rust documentation: $topdir/target/doc/cranelift/index.html"
if has_toolchain nightly; then
    cargo +nightly doc --all --exclude cranelift-codegen-meta
    cargo +nightly doc --package cranelift-codegen-meta --document-private-items

    # Make sure the documentation doesn't have broken links.
    banner "Rust documentation link test"
    ensure_installed cargo-deadlinks
    find ./target/doc -maxdepth 1 -type d -name "cranelift*" | xargs -I{} cargo deadlinks --dir {}
else
    cargo doc --all --exclude cranelift-codegen-meta
    cargo doc --package cranelift-codegen-meta --document-private-items
    echo "nightly toolchain not found, some documentation links will not work"
fi

# Ensure fuzzer works by running it with a single input.
# Note LSAN is disabled due to https://github.com/google/sanitizers/issues/764.
banner "cargo fuzz check"

if ensure_installed cargo-fuzz nightly; then
    fuzz_module="ffaefab69523eb11935a9b420d58826c8ea65c4c"
    ASAN_OPTIONS=detect_leaks=0 \
    cargo +nightly fuzz run fuzz_translate_module \
        "$topdir/fuzz/corpus/fuzz_translate_module/$fuzz_module"
else
    echo "nightly toolchain not found, skipping fuzz target integration test"
fi

banner "OK"
