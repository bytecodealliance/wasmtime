#!/bin/bash
#
# Driver for VeriISLE verification with the SMT query cache.
#
# Usage:
#   ./cranelift/isle/veri/verify.sh [MODE]
#
# Modes:
#   cache-only     Verify purely from the committed cache, in read-only,
#                  enforcing mode. Fails on any cache miss and never invokes an
#                  SMT solver. This is the fast, solver-free check intended for
#                  CI on pull requests.
#
#   rebuild-cache  Regenerate the committed cache: read from the existing cache,
#                  write only the entries actually used into a fresh directory,
#                  then swap it in. Unused entries are dropped (garbage
#                  collected). Cache misses are computed by invoking the SMT
#                  solver, so this requires z3 and/or cvc5 to be installed. This
#                  is intended to run from the main-branch merge queue.
#
#   (no argument)  Local development: use and update the committed cache
#                  in-place. Serves cached results where possible and invokes
#                  the solver on misses, writing new entries back to the cache.
#
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

MODE="${1:-local}"

CACHE_DIR="cranelift/isle/veri/cache"
CONFIGS=(
    cranelift/isle/veri/configs/aarch64-fast.args
    cranelift/isle/veri/configs/aarch64.args
    cranelift/isle/veri/configs/x64-iadd-base-case.args
)

# Run the verifier for every configuration, forwarding the given cache flags.
run_all() {
    for config in "${CONFIGS[@]}"; do
        echo "=== veri: $config ==="
        cargo run -p cranelift-isle-veri --bin veri -- --config "$config" "$@"
    done
}

case "$MODE" in
cache-only)
    echo "=== Verifying from cache (read-only, enforcing; no solver) ==="
    if [ ! -d "$CACHE_DIR" ]; then
        echo "ERROR: cache directory does not exist: $CACHE_DIR" >&2
        exit 1
    fi
    if ! run_all --cache-source-dir "$CACHE_DIR" --cache-mode read-only-enforcing; then
        cat >&2 <<EOF

Verification failed (cache miss or verification error).

A cache miss means the backends changed such that there are new SMT queries not
present in the committed cache. We do not run the SMT solver on pull requests;
instead the solver *results* are committed to the repository.

To regenerate the cache locally (requires z3 and/or cvc5):
  ./cranelift/isle/veri/verify.sh rebuild-cache
Then commit the updated cache under $CACHE_DIR.
EOF
        exit 1
    fi
    echo "=== Cache verification passed ==="
    ;;

rebuild-cache)
    echo "=== Rebuilding cache (read-write; garbage-collects unused entries) ==="
    REBUILD_DIR="$CACHE_DIR.rebuild"
    rm -rf "$REBUILD_DIR"
    mkdir -p "$REBUILD_DIR"
    # Read from the existing cache, write used entries to the fresh directory.
    run_all --cache-source-dir "$CACHE_DIR" \
        --cache-dest-dir "$REBUILD_DIR" \
        --cache-mode read-write
    # Swap the rebuilt cache in.
    rm -rf "$CACHE_DIR"
    mv "$REBUILD_DIR" "$CACHE_DIR"
    echo "=== Cache rebuilt at $CACHE_DIR; review 'git status' and commit ==="
    ;;

local | "")
    echo "=== Verifying with local cache (read-write; in-place) ==="
    run_all --cache-source-dir "$CACHE_DIR" \
        --cache-dest-dir "$CACHE_DIR" \
        --cache-mode read-write
    echo "=== Done ==="
    ;;

*)
    echo "usage: $0 [cache-only | rebuild-cache]" >&2
    echo "       (no argument runs a local, in-place read-write verification)" >&2
    exit 1
    ;;
esac
