#!/bin/sh

# Script to re-vendor the headers from the `wasm-c-api` proposal.

rev="9d6b93764ac96cdd9db51081c363e09d2d488b4d"
files="wasm.h wasm.hh"

set -ex

for file in $files; do
  dst=crates/c-api/include/$file
  pretty_url=https://github.com/WebAssembly/wasm-c-api/blob/$rev/include/$file
  url=https://raw.githubusercontent.com/WebAssembly/wasm-c-api/$rev/include/$file
  cat >$dst <<-EOF
// Wasmtime-vendored copy of this header file as present in upstream:
// <$pretty_url>
//
// Wasmtime maintainers can update this vendored copy in-tree with the
// ./ci/vendor-c-api-headers.sh shell script.
//
EOF
  curl $url >> $dst
done
