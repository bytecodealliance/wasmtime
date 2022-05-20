#!/bin/bash

set -ex

# Determine the name of the tarball
tag=dev
if [[ $GITHUB_REF == refs/tags/v* ]]; then
  tag=${GITHUB_REF:10}
fi
pkgname=wasmtime-$tag-src
rm -rf /tmp/$pkgname
mkdir /tmp/$pkgname

# Vendor all crates.io dependencies since this is supposed to be an
# offline-only-compatible tarball
mkdir /tmp/$pkgname/.cargo
cargo vendor > /tmp/$pkgname/.cargo/config.toml

# Copy over everything in-tree to the destination
cp -r * /tmp/$pkgname

# Create the tarball from the destination
mkdir -p dist
tar -czf dist/$pkgname.tar.gz -C /tmp $pkgname
