#!/usr/bin/env bash

set -exuo pipefail

function reachable() {
    local arch=$1
    cargo run --bin reachable -- \
        --codegen-crate-dir ../../../codegen/ \
        --work-dir /tmp \
        --name "${arch}" \
        > "output/${arch}.reachable"
}

rm -f output/*.reachable

reachable "aarch64"
reachable "x64"
