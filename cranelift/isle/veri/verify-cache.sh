#!/bin/bash
set -euo pipefail
# Runs VeriISLE verification in read-only-enforcing mode against the cache.
# Fails on any cache miss. No solver invocation.
# Usage: ./cranelift/isle/veri/verify-cache.sh [config-file]
cd "$(git rev-parse --show-toplevel)"
CONFIG="${1:-cranelift/isle/veri/configs/aarch64-fast.args}"
CACHE_DIR="cranelift/isle/veri/cache"
echo "=== Verifying from cache (read-only-enforcing) ==="
echo "Config: $CONFIG"
echo "Cache:  $CACHE_DIR"
if [ ! -d "$CACHE_DIR" ]; then
    echo "ERROR: Cache directory does not exist: $CACHE_DIR"
    echo "Run update-cache.sh first to populate the cache."
    exit 1
fi
cargo run -p cranelift-isle-veri --bin veri \
    -- --config "$CONFIG" \
       --cache-dir "$CACHE_DIR" \
       --cache-mode read-only-enforcing
echo "=== Cache verification passed ==="
