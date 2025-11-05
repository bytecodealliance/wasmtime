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
    mkdir -p "$path/$repo"
    cached_extracted_dir="$cache_dir/$repo-$tag"

    if [[ ! -d $cached_extracted_dir ]]; then
      mkdir -p $cached_extracted_dir
      curl --retry 5 --retry-all-errors -sLO https://github.com/WebAssembly/wasi-$repo/archive/$tag.tar.gz
      tar xzf $tag.tar.gz --strip-components=1 -C $cached_extracted_dir
      rm $tag.tar.gz
      rm -rf $cached_extracted_dir/${subdir:-"wit"}/deps*
    fi

    cp -r $cached_extracted_dir/${subdir:-"wit"}/* $path/$repo
  done
}

cache_dir=$(mktemp -d)

make_vendor "wasi-io" "
  io@v0.2.6
"

make_vendor "wasi/src/p2" "
  cli@v0.2.6
  clocks@v0.2.6
  filesystem@v0.2.6
  io@v0.2.6
  random@v0.2.6
  sockets@v0.2.6
"

make_vendor "wasi-http" "
  cli@v0.2.6
  clocks@v0.2.6
  filesystem@v0.2.6
  io@v0.2.6
  random@v0.2.6
  sockets@v0.2.6
  http@v0.2.6
"

make_vendor "wasi-tls" "
  io@v0.2.6
  tls@v0.2.0-draft+505fc98
"

make_vendor "wasi-config" "config@v0.2.0-rc.1"

make_vendor "wasi-keyvalue" "keyvalue@219ea36"

make_vendor "wasi/src/p3" "
    cli@v0.3.0-rc-2025-09-16@wit-0.3.0-draft
    clocks@v0.3.0-rc-2025-09-16@wit-0.3.0-draft
    filesystem@v0.3.0-rc-2025-09-16@wit-0.3.0-draft
    random@v0.3.0-rc-2025-09-16@wit-0.3.0-draft
    sockets@v0.3.0-rc-2025-09-16@wit-0.3.0-draft
"

make_vendor "wasi-http/src/p3" "
    cli@v0.3.0-rc-2025-09-16@wit-0.3.0-draft
    clocks@v0.3.0-rc-2025-09-16@wit-0.3.0-draft
    filesystem@v0.3.0-rc-2025-09-16@wit-0.3.0-draft
    http@v0.3.0-rc-2025-09-16@wit-0.3.0-draft
    random@v0.3.0-rc-2025-09-16@wit-0.3.0-draft
    sockets@v0.3.0-rc-2025-09-16@wit-0.3.0-draft
"

rm -rf $cache_dir

# Separately (for now), vendor the `wasi-nn` WIT files since their retrieval is
# slightly different than above.
repo=https://raw.githubusercontent.com/WebAssembly/wasi-nn
revision=0.2.0-rc-2024-10-28
curl --retry 5 --retry-all-errors -L $repo/$revision/wasi-nn.witx -o crates/wasi-nn/witx/wasi-nn.witx
curl --retry 5 --retry-all-errors -L $repo/$revision/wit/wasi-nn.wit -o crates/wasi-nn/wit/wasi-nn.wit
