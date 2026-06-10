#!/usr/bin/env bash

set -euo pipefail

# Options
function usage() {
    echo "Usage: ${0} -b <bin_dir> [-h] [-v <version>] [-t <tmp_dir>]"
    exit 2
}

version="4.13.0"
bin_dir=""
tmp_dir=""
while getopts "v:b:t:h" opt; do
    case "${opt}" in
        v) version="${OPTARG}" ;;
        b) bin_dir="${OPTARG}" ;;
        t) tmp_dir="${OPTARG}" ;;
        h) usage ;;
        *) usage ;;
    esac
done
shift $((OPTIND-1))

# Check binary install directory.
if [[ ! -d "${bin_dir}" ]]; then
    echo "binary install directory does not exist"
    exit 1
fi

# Setup temp directory.
if [[ -z "${tmp_dir}" ]]; then
    tmp_dir=$(mktemp -d)
fi

if [[ ! -d "${tmp_dir}" ]]; then
    echo "temporary directory does not exist"
    exit 1
fi

pushd "${tmp_dir}"

# Determine which release build.
platform=$(uname -sm)
if [[ "${platform}" == "Darwin arm64" ]]; then
    arch="arm64"
    os="osx-11.0"
elif [[ "${platform}" == "Linux x86_64" ]]; then
    arch="x64"
    os="glibc-2.31"
else
    echo "unsupported platform ${platform}"
    exit 1
fi

# Download.
archive_stem="z3-${version}-${arch}-${os}"
archive_name="${archive_stem}.zip"
url="https://github.com/Z3Prover/z3/releases/download/z3-${version}/${archive_name}"
wget --quiet -O "${archive_name}" "${url}"

# Extract.
unzip "${archive_name}"

# Install.
cp "${archive_stem}/bin/z3" "${bin_dir}"
