#!/usr/bin/env bash

set -euxo pipefail

# Pinned package sources.
#
# asli is pinned to a commit, not a release: the symbolic-EXTR fix (PR #152) is
# merged upstream but not yet in a tagged release.
aslp="https://github.com/UQ-PAC/aslp.git#3b302bd1be7c860888471fdbdf735a5462970d9a"
aslp_rpc="https://github.com/UQ-PAC/aslp-rpc.git#v0.1.4"

switch="${ASLP_SWITCH:-aslp}"

export OPAMYES="true"

# Ensure opam is installed.
if ! command -v opam &> /dev/null; then
    echo "opam is not installed"
    exit 1
fi

# Create the dedicated switch if it does not already exist. aslp_server_http
# needs OCaml >= 5.0; leave the exact 5.x to opam.
if ! opam switch list --short | grep -qx "${switch}"; then
    opam switch create "${switch}" --packages 'ocaml>=5.0' \
        --description "ASLp for Cranelift ISA spec generation"
fi

# Pin upstream sources and install.
opam pin add -n --switch "${switch}" asli             "${aslp}"
opam pin add -n --switch "${switch}" aslp_server_http "${aslp_rpc}"
opam install --switch "${switch}" asli aslp_server_http
