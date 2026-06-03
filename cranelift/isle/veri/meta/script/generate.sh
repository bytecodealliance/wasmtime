#!/usr/bin/env bash

set -euxo pipefail

# Float constants
cargo run --bin fpconst >"../../../codegen/src/fpconst.isle"

# Wasm operators
wasmops="data/wasmoperators.csv"
cargo run --bin wasmoperators >"${wasmops}"

# Translation
wasm2clif="data/wasm2clif.json"
./script/wasm2clif.py --wasm-ops "${wasmops}" --output "${wasm2clif}"

# Tagging
cliftags="data/cliftags.json"
./script/cliftags.py --data "${wasm2clif}" --output "${cliftags}"

# ISLE tags
./script/isletags.py --data "${cliftags}" --output "../../../codegen/src/inst_tags.isle"
