#!/usr/bin/env bash

set -euo pipefail

# Options
function usage() {
    echo "Usage: ${0} [-h] [-a <arch>] [-t <tmp_dir>] [-o <output_dir>] [-p <profile>]"
    exit 2
}

arch="aarch64"
tmp_dir=""
output_dir="output"
profile="dev"
while getopts "a:t:o:p:h" opt; do
    case "${opt}" in
        a) arch="${OPTARG}" ;;
        t) tmp_dir="${OPTARG}" ;;
        o) output_dir="${OPTARG}" ;;
        p) profile="${OPTARG}" ;;
        h) usage ;;
        *) usage ;;
    esac
done
shift $((OPTIND-1))

# Setup output.
mkdir -p "${output_dir}"

# Setup temp directory.
if [[ -z "${tmp_dir}" ]]; then
    tmp_dir=$(mktemp -d)
fi

if [[ ! -d "${tmp_dir}" ]]; then
    echo "temporary directory does not exist"
    exit 1
fi

# Run.
cargo run --bin veri --profile "${profile}" -- \
    --codegen-crate-dir ../../../codegen/ \
    --work-dir "${tmp_dir}" \
    --name "${arch}" \
    --log-dir "${output_dir}/log" \
    "$@" \
    | tee "${output_dir}/${arch}.veri"
