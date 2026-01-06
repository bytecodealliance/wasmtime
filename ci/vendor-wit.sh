#!/usr/bin/env bash

# Script to re-vendor the WIT files that Wasmtime uses using wkg to fetch
# packages from the OCI registry.
#
# This script is executed on CI to ensure that everything is up-to-date.
set -ex

# Temporary directory for downloads
cache_dir=$(mktemp -d)
trap "rm -rf $cache_dir" EXIT

# vendor_wkg fetches packages from the OCI registry using wkg.
# Using --format wit preserves @unstable annotations.
# Each package is placed in its own directory for wit-bindgen compatibility.
#
# Arguments:
#   $1 - Base path for the crate (e.g., "wasi/src/p2")
#   $2 - Space-separated list of packages in format "name@version"
vendor_wkg() {
  local name=$1
  local packages=$2
  local path="crates/$name/wit/deps"

  rm -rf "$path"
  mkdir -p "$path"

  for package in $packages; do
    IFS='@' read -r pkg_name version <<< "$package"
    wkg get "wasi:${pkg_name}@${version}" --format wit --overwrite -o "$path/${pkg_name}.wit"
  done
}

# vendor_github fetches packages from GitHub tarballs for packages not
# available in the OCI registry.
#
# Arguments:
#   $1 - Base path for the crate (e.g., "wasi-tls")
#   $2 - Space-separated list of packages in format "name@tag[@subdir]"
vendor_github() {
  local name=$1
  local packages=$2
  local path="crates/$name/wit/deps"

  rm -rf "$path"
  mkdir -p "$path"

  for package in $packages; do
    IFS='@' read -r repo tag subdir <<< "$package"
    mkdir -p "$path/$repo"
    local extracted_dir="$cache_dir/$repo-$tag"

    if [[ ! -d $extracted_dir ]]; then
      mkdir -p "$extracted_dir"
      curl --retry 5 --retry-all-errors -sLO "https://github.com/WebAssembly/wasi-$repo/archive/$tag.tar.gz"
      tar xzf "$tag.tar.gz" --strip-components=1 -C "$extracted_dir"
      rm "$tag.tar.gz"
      rm -rf "$extracted_dir/${subdir:-wit}/deps"*
    fi

    cp -r "$extracted_dir/${subdir:-wit}"/* "$path/$repo"
  done
}

# WASI Preview 2 packages (0.2.6)
vendor_wkg "wasi-io" "io@0.2.6"

vendor_wkg "wasi/src/p2" "
  cli@0.2.6
  clocks@0.2.6
  filesystem@0.2.6
  io@0.2.6
  random@0.2.6
  sockets@0.2.6
"

vendor_wkg "wasi-http" "
  cli@0.2.6
  clocks@0.2.6
  filesystem@0.2.6
  io@0.2.6
  random@0.2.6
  sockets@0.2.6
  http@0.2.6
"

# wasi-tls is not yet published to OCI registry, use GitHub
vendor_github "wasi-tls" "
  io@v0.2.6
  tls@v0.2.0-draft+505fc98
"

# wasi-config and wasi-keyvalue from OCI registry
vendor_wkg "wasi-config" "config@0.2.0-rc.1"
vendor_wkg "wasi-keyvalue" "keyvalue@0.2.0-draft"

# WASI Preview 3 packages (0.3.0-rc-2026-01-06)
vendor_wkg "wasi/src/p3" "
  cli@0.3.0-rc-2026-01-06
  clocks@0.3.0-rc-2026-01-06
  filesystem@0.3.0-rc-2026-01-06
  random@0.3.0-rc-2026-01-06
  sockets@0.3.0-rc-2026-01-06
"

vendor_wkg "wasi-http/src/p3" "
  cli@0.3.0-rc-2026-01-06
  clocks@0.3.0-rc-2026-01-06
  filesystem@0.3.0-rc-2026-01-06
  http@0.3.0-rc-2026-01-06
  random@0.3.0-rc-2026-01-06
  sockets@0.3.0-rc-2026-01-06
"

# wasi-nn is fetched separately since it's not in the standard WASI registry
repo=https://raw.githubusercontent.com/WebAssembly/wasi-nn
revision=0.2.0-rc-2024-10-28
curl --retry 5 --retry-all-errors -L "$repo/$revision/wasi-nn.witx" -o crates/wasi-nn/witx/wasi-nn.witx
curl --retry 5 --retry-all-errors -L "$repo/$revision/wit/wasi-nn.wit" -o crates/wasi-nn/wit/wasi-nn.wit
