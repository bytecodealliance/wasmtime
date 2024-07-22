#!/usr/bin/env bash

# Script to re-vendor the WIT files that Wasmtime uses as defined by a
# particular tag in upstream repositories.
#
# This script is executed on CI to ensure that everything is up-to-date.
set -ex

# The make_vendor function takes a base path (e.g., "wasi") and a list
# of packages in the format "name@tag". It constructs the full destination
# path, downloads the tarballs from GitHub, extracts the relevant files, and
# removes any unwanted directories.
make_vendor() {
  local name=$1
  local packages=$2
  local path="crates/$name/wit/deps"

  rm -rf $path
  mkdir -p $path

  for package in $packages; do
    IFS='@' read -r repo tag <<< "$package"
    mkdir -p $path/$repo
    cached_extracted_dir="$cache_dir/$repo-$tag"

    if [[ ! -d $cached_extracted_dir ]]; then
      mkdir -p $cached_extracted_dir
      curl -sL https://github.com/WebAssembly/wasi-$repo/archive/$tag.tar.gz | \
        tar xzf - --strip-components=1 -C $cached_extracted_dir
      rm -rf $cached_extracted_dir/wit/deps*
    fi

    cp -r $cached_extracted_dir/wit/* $path/$repo
  done
}

cache_dir=$(mktemp -d)

make_vendor "wasi" "
  cli@v0.2.0
  clocks@v0.2.0
  filesystem@v0.2.0
  io@v0.2.0
  random@v0.2.0
  sockets@v0.2.0
  http@v0.2.0
"

make_vendor "wasi-http" "
  cli@v0.2.0
  clocks@v0.2.0
  filesystem@v0.2.0
  io@v0.2.0
  random@v0.2.0
  sockets@v0.2.0
  http@v0.2.0
"

make_vendor "wasi-runtime-config" "runtime-config@c667fe6"

make_vendor "wasi-keyvalue" "keyvalue@219ea36"

rm -rf $cache_dir

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
