#!/usr/bin/env bash

set -exuo pipefail

function count() {
    cargo run --bin count -- \
        --codegen-crate-dir ../../../codegen/ \
        --work-dir /tmp \
        "$@"
}

rm -f output/*.count

count \
    --name "aarch64" \
    --term-name lower \
    --max-rules 3 \
    --exclude-chain operand_size \
    > "output/aarch64_lower.count"
