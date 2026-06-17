#!/usr/bin/env bash

set -euxo pipefail

# Options
function usage() {
    echo "Usage: ${0} -i <install_dir> [-h] [-v <version>] [-t <tmp_dir>]"
    exit 2
}

version="1.2.0"
install_dir=""
tmp_dir=""
while getopts "v:i:t:h" opt; do
    case "${opt}" in
        v) version="${OPTARG}" ;;
        i) install_dir="${OPTARG}" ;;
        t) tmp_dir="${OPTARG}" ;;
        h) usage ;;
        *) usage ;;
    esac
done
shift $((OPTIND-1))

# Check install directory.
if [[ ! -d "${install_dir}" ]]; then
    echo "install directory does not exist"
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
    os="macOS"
    mode="shared"
elif [[ "${platform}" == "Linux x86_64" ]]; then
    arch="x86_64"
    os="Linux"
    mode="static"
else
    echo "unsupported platform ${platform}"
    exit 1
fi

# Download.
archive_stem="cvc5-${os}-${arch}-${mode}"
archive_name="${archive_stem}.zip"
url="https://github.com/cvc5/cvc5/releases/download/cvc5-${version}/${archive_name}"
wget --quiet -O "${archive_name}" "${url}"

# Extract.
unzip "${archive_name}"

# Install.
cp -r "${archive_stem}"/* "${install_dir}/"
