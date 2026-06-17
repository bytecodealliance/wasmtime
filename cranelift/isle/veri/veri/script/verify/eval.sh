#!/usr/bin/env bash

set -euo pipefail

# Options.
function usage() {
    echo "Usage: ${0} -n <name> -t <timeout>"
    exit 2
}

name="adhoc"
timeout=60
mode="full"
while getopts "n:t:c" opt; do
    case "${opt}" in
        n) name="${OPTARG}" ;;
        t) timeout="${OPTARG}" ;;
        c) mode="ci" ;;
        *) usage ;;
    esac
done

[[ -n "${name}" ]]
[[ -n "${EVAL_DATA_DIR-data}" ]]

# Metadata helpers.
function json_new() {
    local file="${1}"
    echo '{}' >"${file}"
}

function json_set() {
    local file="${1}"
    local key="${2}"
    local value="${3}"
    jq '. += $ARGS.named' --arg "${key}" "${value}" "${file}" | sponge "${file}"
}

# Setup temporary directory.
tmp_dir=$(mktemp -d)

# Setup results directory.
timestamp=$(date -u '+%Y-%m-%dT%T')
output_dir="${EVAL_DATA_DIR-data}/run/${timestamp}-${name}"
mkdir -p "${output_dir}"

# Save metadata
metadata_file="${output_dir}/metadata.json"
json_new "${metadata_file}"
json_set "${metadata_file}" "name" "${name}"
json_set "${metadata_file}" "timestamp" "${timestamp}"
json_set "${metadata_file}" "timeout" "${timeout}"
json_set "${metadata_file}" "hostname" "$(hostname)"

z3_version=$(z3 --version)
json_set "${metadata_file}" "z3_version" "${z3_version}"

cvc5_version=$(cvc5 --version | head -n 1)
json_set "${metadata_file}" "cvc5_version" "${cvc5_version}"

# System information.
system_dir="${output_dir}/sys/"
mkdir -p "${system_dir}"
lscpu >"${system_dir}/lscpu.out"
cp /proc/cpuinfo "${system_dir}/cpuinfo"

# Clean build
cargo clean

# Eval
extra_args=()
case "${mode}" in
    "ci") extra_args+=("--ignore-solver-tags") ;;
esac

RUST_LOG=info \
cargo run --bin veri --release -- \
    --codegen-crate-dir ../../../codegen/ \
    --work-dir "${tmp_dir}" \
    --name aarch64 \
    --log-dir "${output_dir}/log" \
    --results-to-log-dir \
    --timeout "${timeout}" \
    --num-threads 0 \
    --no-skip-todo \
    "${extra_args[@]}" \
    \
    --filter include:tag:wasm_proposal_mvp \
    --filter exclude:tag:wasm_category_stack \
    --filter exclude:not:root:lower \
    --filter exclude:tag:vector \
    --filter exclude:tag:atomics \
    --filter exclude:tag:spectre \
    --filter exclude:tag:narrowfloat \
    --filter include:tag:clif_popcnt \
    --filter exclude:tag:amode_const \
    --filter exclude:tag:i128 \
    \
    --filter include:root:emit_side_effect \
    --filter include:root:operand_size \
    --filter include:root:scalar_size \
    --filter include:root:size_from_ty \
    ;
