#!/usr/bin/env bash

set -exuo pipefail

# Options
function usage() {
    echo "Usage: ${0} [-h] [-o <output_dir>] [-p <port>]"
    exit 2
}

output_dir="${ISLE_EXPLORER_OUTPUT_DIR:-}"
port="5050"
while getopts "o:p:h" opt; do
    case "${opt}" in
        o) output_dir="${OPTARG}" ;;
        p) port="${OPTARG}" ;;
        h) usage ;;
        *) usage ;;
    esac
done

if [[ ! -d "${output_dir}" ]]; then
    echo "output directory does not exist"
    exit 1
fi

# Serve.
miniserve \
    --port "${port}" \
    --index index.html \
    --disable-indexing \
    --verbose \
    "${output_dir}"
