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
if ! cargo run -p cranelift-isle-veri --bin veri \
    -- --config "$CONFIG" \
       --cache-dir "$CACHE_DIR" \
       --cache-mode read-only-enforcing; then
    echo ""
    echo "Cranelift verification failed (cache miss or verification error)."
    echo ""
    echo "This may be because you made a change to the backends and there are"
    echo "new SMT queries that must be run for verification to complete."
    echo ""
    echo "Note that we do not run the SMT solver in CI; you need to run it locally,"
    echo "then commit the SMT solver *result* to the repo (which we trust, and may"
    echo "verify for new contributors)."
    echo ""
    echo "To re-populate the cache locally, run:"
    echo "  ./cranelift/isle/veri/update-cache.sh $CONFIG"
    echo "Then commit the new cache entries."
    exit 1
fi
echo "=== Cache verification passed ==="
