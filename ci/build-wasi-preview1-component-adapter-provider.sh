#!/usr/bin/env bash
set -ex

# Manifest the adapter provider into the workspace
cp crates/wasi-preview1-component-adapter/provider/Cargo.toml.in crates/wasi-preview1-component-adapter/provider/Cargo.toml
sed -i '/"crates\/wasi-preview1-component-adapter",/a\ \ "crates\/wasi-preview1-component-adapter\/provider",' Cargo.toml

# Check the adapter provider's code formatting and style
cargo fmt -p wasi-preview1-component-adapter-provider -- --check
cargo check -p wasi-preview1-component-adapter-provider
cargo clippy -p wasi-preview1-component-adapter-provider

# Check that publishing the adapter provider should work
cargo publish -p wasi-preview1-component-adapter-provider --dry-run --allow-dirty
