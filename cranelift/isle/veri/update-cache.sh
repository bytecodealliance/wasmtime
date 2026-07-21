#!/bin/bash
set -euo pipefail
# Runs VeriISLE verification in read-write mode, updating the cache.
# Usage: ./cranelift/isle/veri/update-cache.sh [config-file]
cd "$(git rev-parse --show-toplevel)"
CONFIG="${1:-cranelift/isle/veri/configs/aarch64-fast.args}"
CACHE_DIR="cranelift/isle/veri/cache"
echo "=== Updating cache in read-write mode ==="
echo "Config: $CONFIG"
echo "Cache:  $CACHE_DIR"
cargo run -p cranelift-isle-veri --bin veri \
    -- --config "$CONFIG" \
       --cache-dir "$CACHE_DIR" \
       --cache-mode read-write
echo "=== Done ==="
