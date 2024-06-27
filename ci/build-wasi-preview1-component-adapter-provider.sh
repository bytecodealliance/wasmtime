#!/usr/bin/env bash
set -ex

# Manifest the adapter provider into the workspace
cp crates/wasi-preview1-component-adapter/provider/Cargo.toml.in crates/wasi-preview1-component-adapter/provider/Cargo.toml
sed -i '/"crates\/wasi-preview1-component-adapter",/a\ \ "crates\/wasi-preview1-component-adapter\/provider",' Cargo.toml

set +x
if [ "$CHECK" = "1" ]; then
  cargo fmt -p wasi-preview1-component-adapter-provider -- --check
  cargo check -p wasi-preview1-component-adapter-provider
  cargo clippy -p wasi-preview1-component-adapter-provider
  cargo publish -p wasi-preview1-component-adapter-provider --dry-run --allow-dirty
fi
set -x
