#!/usr/bin/env bash

set -euxo pipefail

# Defaults.
repo="mmcloughlin/aslp"
version="8ca39c0f1b7f4b588fc840aa1a9fbbc5b9085ad0"
ocaml_compiler="4.14.2"

# Options
function usage() {
    echo "Usage: ${0} -i <install_dir> [-h] [-r <repo>] [-v <version>] [-c <ocaml_compiler>] [-t <tmp_dir>]"
    exit 2
}

install_dir=""
tmp_dir=""
while getopts "r:v:c:i:t:h" opt; do
    case "${opt}" in
        r) repo="${OPTARG}" ;;
        v) version="${OPTARG}" ;;
        c) ocaml_compiler="${OPTARG}" ;;
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

# Ensure opam is installed.
if ! command -v opam &> /dev/null; then
    echo "opam is not installed"
    exit 1
fi

# # Setup opam root.
export OPAMROOT="${install_dir}/.opam"
export OPAMYES="true"
opam init --compiler="${ocaml_compiler}"

eval $(opam env)

# Download and extract ASLp.
archive_name="${version}.tar.gz"
archive_url="https://github.com/${repo}/archive/${version}.tar.gz"
wget --quiet "${archive_url}"

tar xvzf \
    "${archive_name}" \
    --strip-components=1

# Install.
export DUNE_INSTALL_PREFIX="${install_dir}"
opam install . --deps-only --with-test
opam exec -- dune build
opam exec -- dune install
