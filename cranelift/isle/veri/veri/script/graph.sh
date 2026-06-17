#!/usr/bin/env bash

set -exuo pipefail

# Clean.
rm -f output/*.{dot,svg}

# Rules.
arch="aarch64"
rules=(
    "iadd_base_case"
    "iadd_imm12_right"
    "iadd_imm12_left"
    "iadd_i128"
)

for rule in "${rules[@]}"; do
    name="${arch}_${rule}"

    # Generate dot.
    dot_path="output/${name}.dot"
    cargo run --bin graph -- \
        --codegen-crate-dir ../../../codegen/ \
        --work-dir /tmp \
        --name "${arch}" \
        --rule "${rule}" \
        | tee "output/${name}.dot"

    # Render.
    svg_path="output/${name}.svg"
    dot -Tsvg "${dot_path}" >"${svg_path}"
done
