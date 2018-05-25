#!/bin/bash
set -euo pipefail

# Format all sources using rustfmt.

cd $(dirname "$0")

# Make sure we can find rustfmt.
export PATH="$PATH:$HOME/.cargo/bin"

exec cargo +stable fmt --all -- "$@"
