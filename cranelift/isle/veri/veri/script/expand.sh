#!/usr/bin/env bash

set -exuo pipefail

function expand() {
    cargo run --bin expand -- \
        --codegen-crate-dir ../../../codegen/ \
        --work-dir /tmp \
        "$@"
}

rm -f output/*.out

expand \
    --name aarch64 \
    --term-name sink_load_into_addr \
    > output/sink_load_into_addr.out

expand \
    --name aarch64 \
    --term-name sink_load_into_addr \
    --chain add_imm_to_addr \
    > output/sink_load_into_addr_inline_add_imm_to_addr.out

expand \
    --name aarch64 \
    --term-name sink_load_into_addr \
    --chain add_imm_to_addr \
    --chain add_imm \
    > output/sink_load_into_addr_inline_add_imm_to_addr_add_imm.out

expand \
    --name aarch64 \
    --term-name sink_load_into_addr \
    --maximal-chaining \
    > output/sink_load_into_addr_maximal_inlining.out

expand \
    --name aarch64 \
    --term-name lower \
    > output/lower.out

expand \
    --name aarch64 \
    --term-name lower \
    --no-expand-internal-extractors \
    > output/lower_internal_extractors.out

expand \
    --name aarch64 \
    --term-name lower \
    --no-expand-internal-extractors \
    --maximal-chaining \
    --max-rules 1 \
    > output/lower_internal_extractors_maximal_inline_1.out

expand \
    --name aarch64 \
    --term-name lower \
    --no-expand-internal-extractors \
    --maximal-chaining \
    --max-rules 2 \
    --exclude-chain operand_size \
    > output/lower_internal_extractors_maximal_inline_2.out

expand \
    --name aarch64 \
    --term-name lower \
    --no-expand-internal-extractors \
    --maximal-chaining \
    --max-rules 3 \
    --exclude-chain operand_size \
    > output/lower_internal_extractors_maximal_inline_3.out

expand \
    --name aarch64 \
    --term-name lower \
    --no-expand-internal-extractors \
    --maximal-chaining \
    --max-rules 3 \
    --exclude-chain operand_size \
    --no-prune-infeasible \
    > output/lower_internal_extractors_no_prune_maximal_inline_3.out

expand \
    --name aarch64 \
    --term-name lower \
    --no-expand-internal-extractors \
    --maximal-chaining \
    --max-rules 6 \
    --exclude-chain operand_size \
    > output/lower_internal_extractors_maximal_inline_6.out

expand \
    --name x64 \
    --term-name lower \
    --no-expand-internal-extractors \
    > output/x64_lower_internal_extractors.out

expand \
    --name x64 \
    --term-name lower \
    --no-expand-internal-extractors \
    --chain to_amode_add \
    --chain amode_imm_reg_reg_shift \
    > output/x64_lower_internal_extractors_amode_inlining.out
