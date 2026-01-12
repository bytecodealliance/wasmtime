#!/usr/bin/env bash

# Script to re-vendor the WIT files that Wasmtime uses using wkg to fetch
# packages from the OCI registry.
#
# This script is executed on CI to ensure that everything is up-to-date.
set -ex

# Temporary directory for downloads
cache_dir=$(mktemp -d)
trap "rm -rf $cache_dir" EXIT

# Helper to download the `WebAssembly/$repo` dir at the `$tag` (or rev)
# specified. The `wit/*.wit` files are placed in `$path`.
get_github() {
  local repo=$1
  local tag=$2
  local path=$3

  rm -rf "$path"
  mkdir -p "$path"

  cached_extracted_dir="$cache_dir/$repo-$tag"

  if [[ ! -d $cached_extracted_dir ]]; then
    mkdir -p $cached_extracted_dir
    curl --retry 5 --retry-all-errors -sLO https://github.com/WebAssembly/$repo/archive/$tag.tar.gz
    tar xzf $tag.tar.gz --strip-components=1 -C $cached_extracted_dir
    rm $tag.tar.gz
    rm -rf $cached_extracted_dir/wit/deps*
  fi

  cp -r $cached_extracted_dir/wit/* $path
}

p2=0.2.6
p3=0.3.0-rc-2025-09-16

rm -rf crates/wasi-io/wit/deps
mkdir -p crates/wasi-io/wit/deps
wkg get --format wit --overwrite "wasi:io@$p2" -o "crates/wasi-io/wit/deps/io.wit"

rm -rf crates/wasi/src/p2/wit/deps
mkdir -p crates/wasi/src/p2/wit/deps
wkg get --format wit --overwrite "wasi:io@$p2" -o "crates/wasi/src/p2/wit/deps/io.wit"
wkg get --format wit --overwrite "wasi:clocks@$p2" -o "crates/wasi/src/p2/wit/deps/clocks.wit"
wkg get --format wit --overwrite "wasi:cli@$p2" -o "crates/wasi/src/p2/wit/deps/cli.wit"
wkg get --format wit --overwrite "wasi:filesystem@$p2" -o "crates/wasi/src/p2/wit/deps/filesystem.wit"
wkg get --format wit --overwrite "wasi:random@$p2" -o "crates/wasi/src/p2/wit/deps/random.wit"
wkg get --format wit --overwrite "wasi:sockets@$p2" -o "crates/wasi/src/p2/wit/deps/sockets.wit"

rm -rf crates/wasi-http/wit/deps
mkdir -p crates/wasi-http/wit/deps
wkg get --format wit --overwrite "wasi:io@$p2" -o "crates/wasi-http/wit/deps/io.wit"
wkg get --format wit --overwrite "wasi:clocks@$p2" -o "crates/wasi-http/wit/deps/clocks.wit"
wkg get --format wit --overwrite "wasi:cli@$p2" -o "crates/wasi-http/wit/deps/cli.wit"
wkg get --format wit --overwrite "wasi:filesystem@$p2" -o "crates/wasi-http/wit/deps/filesystem.wit"
wkg get --format wit --overwrite "wasi:random@$p2" -o "crates/wasi-http/wit/deps/random.wit"
wkg get --format wit --overwrite "wasi:sockets@$p2" -o "crates/wasi-http/wit/deps/sockets.wit"
wkg get --format wit --overwrite "wasi:http@$p2" -o "crates/wasi-http/wit/deps/http.wit"


rm -rf crates/wasi-tls/wit/deps
mkdir -p crates/wasi-tls/wit/deps
wkg get --format wit --overwrite "wasi:io@$p2" -o "crates/wasi-tls/wit/deps/io.wit"
get_github wasi-tls v0.2.0-draft+505fc98 crates/wasi-tls/wit/deps/tls

rm -rf crates/wasi-config/wit/deps
mkdir -p crates/wasi-config/wit/deps
get_github wasi-config v0.2.0-rc.1 crates/wasi-config/wit/deps/config

rm -rf crates/wasi-keyvalue/wit/deps
mkdir -p crates/wasi-keyvalue/wit/deps
get_github wasi-keyvalue 219ea36 crates/wasi-keyvalue/wit/deps/keyvalue

rm -rf crates/wasi/src/p3/wit/deps
mkdir -p crates/wasi/src/p3/wit/deps
wkg get --format wit --overwrite "wasi:clocks@$p3" -o "crates/wasi/src/p3/wit/deps/clocks.wit"
wkg get --format wit --overwrite "wasi:cli@$p3" -o "crates/wasi/src/p3/wit/deps/cli.wit"
wkg get --format wit --overwrite "wasi:filesystem@$p3" -o "crates/wasi/src/p3/wit/deps/filesystem.wit"
wkg get --format wit --overwrite "wasi:random@$p3" -o "crates/wasi/src/p3/wit/deps/random.wit"
wkg get --format wit --overwrite "wasi:sockets@$p3" -o "crates/wasi/src/p3/wit/deps/sockets.wit"

rm -rf crates/wasi-http/src/p3/wit/deps
mkdir -p crates/wasi-http/src/p3/wit/deps
wkg get --format wit --overwrite "wasi:clocks@$p3" -o "crates/wasi-http/src/p3/wit/deps/clocks.wit"
wkg get --format wit --overwrite "wasi:cli@$p3" -o "crates/wasi-http/src/p3/wit/deps/cli.wit"
wkg get --format wit --overwrite "wasi:filesystem@$p3" -o "crates/wasi-http/src/p3/wit/deps/filesystem.wit"
wkg get --format wit --overwrite "wasi:random@$p3" -o "crates/wasi-http/src/p3/wit/deps/random.wit"
wkg get --format wit --overwrite "wasi:sockets@$p3" -o "crates/wasi-http/src/p3/wit/deps/sockets.wit"
wkg get --format wit --overwrite "wasi:http@$p3" -o "crates/wasi-http/src/p3/wit/deps/http.wit"

# wasi-nn is fetched separately since it's not in the standard WASI registry
repo=https://raw.githubusercontent.com/WebAssembly/wasi-nn
revision=0.2.0-rc-2024-10-28
curl --retry 5 --retry-all-errors -L "$repo/$revision/wasi-nn.witx" -o crates/wasi-nn/witx/wasi-nn.witx
curl --retry 5 --retry-all-errors -L "$repo/$revision/wit/wasi-nn.wit" -o crates/wasi-nn/wit/wasi-nn.wit
