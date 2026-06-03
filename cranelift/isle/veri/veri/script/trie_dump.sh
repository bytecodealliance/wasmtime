#!/usr/bin/env bash

set -exuo pipefail

function trie_dump() {
    local arch=$1
    cargo run --bin trie_dump -- \
        --codegen-crate-dir ../../../codegen/ \
        --work-dir /tmp \
        --name "${arch}" \
        > "output/${arch}.trie"
}

trie_dump "aarch64"
trie_dump "x64"
