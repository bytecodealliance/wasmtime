#!/usr/bin/env bash

set -exuo pipefail

function rule_stats() {
    local arch=$1
    cargo run --bin rule_stats -- \
        --codegen-crate-dir ../../../codegen/ \
        --work-dir /tmp \
        --name "${arch}" \
        > "output/${arch}.stats"
}

rule_stats "aarch64"
rule_stats "x64"
