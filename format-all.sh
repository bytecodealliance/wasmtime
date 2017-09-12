#!/bin/bash

# Format all sources using rustfmt.

# Exit immediately on errors.
set -e

cd $(dirname "$0")

# Make sure we can find rustfmt.
export PATH="$PATH:$HOME/.cargo/bin"

exec cargo fmt --all -- "$@"
