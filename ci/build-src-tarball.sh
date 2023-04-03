#!/bin/bash

set -ex

# Determine the name of the tarball
tag=dev
if [[ $GITHUB_REF == refs/heads/release-* ]]; then
  tag=v${GITHUB_REF:19}
fi
pkgname=wasmtime-$tag-src

# Vendor all crates.io dependencies since this is supposed to be an
# offline-only-compatible tarball
mkdir .cargo
cargo vendor > .cargo/config.toml

# Create the tarball from the destination
tar -czf /tmp/$pkgname.tar.gz --transform "s/^\./$pkgname/S" --exclude=.git .
mkdir -p dist
mv /tmp/$pkgname.tar.gz dist/
