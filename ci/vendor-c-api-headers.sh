#!/bin/sh

# Script to re-vendor the headers from the `wasm-c-api` proposal.

rev="2ce1367c9d1271c83fb63bef26d896a2f290cd23"
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
