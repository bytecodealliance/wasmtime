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
  cli@v0.2.2
  clocks@v0.2.2
  filesystem@v0.2.2
  io@v0.2.2
  random@v0.2.2
  sockets@v0.2.2
"

make_vendor "wasi-http" "
  cli@v0.2.2
  clocks@v0.2.2
  filesystem@v0.2.2
  io@v0.2.2
  random@v0.2.2
  sockets@v0.2.2
  http@v0.2.2
"

make_vendor "wasi-config" "config@f4d699b"

make_vendor "wasi-keyvalue" "keyvalue@219ea36"

rm -rf $cache_dir

# Separately (for now), vendor the `wasi-nn` WIT files since their retrieval is
# slightly different than above.
repo=https://raw.githubusercontent.com/WebAssembly/wasi-nn
revision=0.2.0-rc-2024-08-19
curl -L $repo/$revision/wasi-nn.witx -o crates/wasi-nn/witx/wasi-nn.witx
curl -L $repo/$revision/wit/wasi-nn.wit -o crates/wasi-nn/wit/wasi-nn.wit
