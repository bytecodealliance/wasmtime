#!/usr/bin/env bash

set -exuo pipefail

# Options
function usage() {
    echo "Usage: ${0} [-h] [-l] [-a <aslp_server_host>] [-p <aslp_server_port>] [-o <output_path>]"
    exit 2
}

launch_server="false"
aslp_server_host="${ASLP_SERVER_HOST:-127.0.0.1}"
aslp_server_port="${ASLP_SERVER_PORT:-4207}"
output_path="../../../codegen/src/isa/aarch64/spec/"
while getopts "la:p:o:h" opt; do
    case "${opt}" in
        l) launch_server="true" ;;
        a) aslp_server_host="${OPTARG}" ;;
        p) aslp_server_port="${OPTARG}" ;;
        o) output_path="${OPTARG}" ;;
        h) usage ;;
        *) usage ;;
    esac
done

# Floating-point constant specs.
cargo run --bin fpconst > "${output_path}/fp_const.isle"

# Launch server
if [[ "${launch_server}" == "true" ]]; then
    aslp-server --host "${aslp_server_host}" --port "${aslp_server_port}" &
    aslp_server_pid=$!
    trap 'kill "${aslp_server_pid}"' EXIT
fi

# Generate
aslp_server_url="http://${aslp_server_host}:${aslp_server_port}"
cargo run --bin isaspec \
    -- \
    --server "${aslp_server_url}" \
    --output "${output_path}"
