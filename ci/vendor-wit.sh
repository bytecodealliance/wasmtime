#!/bin/sh

# Script to re-vendor the WIT files that Wasmtime uses as defined by a
# particular tag in upstream repositories.
#
# This script is executed on CI to ensure that everything is up-to-date.
set -ex

# Space-separated list of wasi proposals that are vendored here along with the
# tag that they're all vendored at.
#
# This assumes that the repositories all have the pattern:
# https://github.com/WebAssembly/wasi-$repo
# and every repository has a tag `v$tag` here. That is currently done as part
# of the WASI release process.
repos="cli clocks filesystem http io random sockets"
tag=0.2.0

# First, replace the existing vendored WIT files in the `wasi` crate.
dst=crates/wasi/wit/deps
rm -rf $dst
mkdir -p $dst
for repo in $repos; do
  mkdir $dst/$repo
  curl -L https://github.com/WebAssembly/wasi-$repo/archive/refs/tags/v$tag.tar.gz | \
    tar xzf - --strip-components=2 -C $dst/$repo wasi-$repo-$tag/wit
  rm -rf $dst/$repo/deps*
done

# Also replace the `wasi-http` WIT files since they match those in the `wasi`
# crate.
rm -rf crates/wasi-http/wit/deps
cp -r $dst crates/wasi-http/wit

rm -rf crates/test-programs/wit/deps
cp -r $dst crates/test-programs/wit

# Separately (for now), vendor the `wasi-nn` WIT files since their retrieval is
# slightly different than above.
repo=https://raw.githubusercontent.com/WebAssembly/wasi-nn
revision=e2310b
curl -L $repo/$revision/wasi-nn.witx -o crates/wasi-nn/witx/wasi-nn.witx
# TODO: the in-tree `wasi-nn` implementation does not yet fully support the
# latest WIT specification on `main`. To create a baseline for moving forward,
# the in-tree WIT incorporates some but not all of the upstream changes. This
# TODO can be removed once the implementation catches up with the spec.
# curl -L $repo/$revision/wit/wasi-nn.wit -o crates/wasi-nn/wit/wasi-nn.wit
