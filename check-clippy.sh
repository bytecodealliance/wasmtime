#!/bin/bash
set -euo pipefail

# Usage: check-clippy.sh

if cargo install --list | tee /dev/null | grep -q "^clippy v0"; then
    exit 0
else
    exit 1
fi
