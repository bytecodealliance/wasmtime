#!/bin/bash

set -ex

# Determine the name of the tarball
tag=dev
if [[ $GITHUB_REF == refs/tags/v* ]]; then
  tag=${GITHUB_REF#refs/tags/}
fi
pkgname=wasmtime-$tag-src
rm -rf /tmp/$pkgname
mkdir /tmp/$pkgname

# Vendor all crates.io dependencies since this is supposed to be an
# offline-only-compatible tarball
mkdir /tmp/$pkgname/.cargo
cargo vendor > /tmp/$pkgname/.cargo/config.toml

# Create the tarball from the destination
tar -czf /tmp/$pkgname.tar.gz --transform "s/^\./$pkgname/S" --exclude-vcs .
mkdir -p dist
mv /tmp/$pkgname.tar.gz dist/
