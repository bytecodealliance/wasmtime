#!/bin/sh

# Script to re-vendor the WIT files that Wasmtime uses as defined by a
# particular tag in upstream repositories.
#
# This script is executed on CI to ensure that everything is up-to-date.
set -ex

# Check if the command exists
if ! command -v "wit-deps" &> /dev/null; then
    echo "wit-deps not found, installing..."
    cargo install wit-deps-cli
fi

wit-deps --manifest crates/wasi/wit/deps.toml --lock crates/wasi/wit/deps.lock --deps crates/wasi/wit/deps
wit-deps --manifest crates/wasi-http/wit/deps.toml --lock crates/wasi-http/wit/deps.lock --deps crates/wasi-http/wit/deps

# TODO: the in-tree `wasi-nn` implementation does not yet fully support the
# latest WIT specification on `main`. To create a baseline for moving forward,
# the in-tree WIT incorporates some but not all of the upstream changes.
# Once the implementation catches up with the spec, this TODO can be removed
# and wit-deps can be used to manage wit dependencies.

# Vendor the `wasi-nn` WITX files.
repo=https://raw.githubusercontent.com/WebAssembly/wasi-nn
revision=e2310b
curl -L $repo/$revision/wasi-nn.witx -o crates/wasi-nn/witx/wasi-nn.witx
