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
    IFS='@' read -r repo tag subdir <<< "$package"
    mkdir -p "$path/$package"
    cached_extracted_dir="$cache_dir/$repo-$tag"

    if [[ ! -d $cached_extracted_dir ]]; then
      mkdir -p $cached_extracted_dir
      curl -sL https://github.com/WebAssembly/wasi-$repo/archive/$tag.tar.gz | \
        tar xzf - --strip-components=1 -C $cached_extracted_dir
      rm -rf $cached_extracted_dir/${subdir:-"wit"}/deps*
    fi

    cp -r $cached_extracted_dir/${subdir:-"wit"}/* $path/$package
  done
}

cache_dir=$(mktemp -d)

make_vendor "wasi/src/p2" "
  cli@v0.2.3
  clocks@v0.2.3
  filesystem@v0.2.3
  io@v0.2.3
  random@v0.2.3
  sockets@v0.2.3
"

make_vendor "wasi/src/p3" "
  clocks@3850f9d@wit-0.3.0-draft
  random@3e99124@wit-0.3.0-draft
"

make_vendor "wasi-http/src/p2" "
  cli@v0.2.3
  clocks@v0.2.3
  filesystem@v0.2.3
  io@v0.2.3
  random@v0.2.3
  sockets@v0.2.3
  http@v0.2.3
"

make_vendor "wasi-config/src/p2" "config@f4d699b"

make_vendor "wasi-keyvalue/src/p2" "keyvalue@219ea36"

rm -rf $cache_dir

# Separately (for now), vendor the `wasi-nn` WIT files since their retrieval is
# slightly different than above.
repo=https://raw.githubusercontent.com/WebAssembly/wasi-nn
revision=0.2.0-rc-2024-08-19
curl -L $repo/$revision/wasi-nn.witx -o crates/wasi-nn/witx/wasi-nn.witx
curl -L $repo/$revision/wit/wasi-nn.wit -o crates/wasi-nn/wit/wasi-nn.wit
