#!/bin/bash
#
# Driver for VeriISLE verification with the SMT query cache.
#
# The cache lives locally at cranelift/isle/veri/cache (gitignored). CI
# (the isle_veri_full_check job in .github/workflows/main.yml) verifies on
# top of the shared entry in the GitHub Actions cache (keyed by a hash of
# the ISLE sources and toolchain) and also uploads the rebuilt cache as a
# run artifact, which the publish-artifacts.yml workflow publishes as
# isle-veri-cache.tar.gz on the `dev` release when the run lands on main,
# for local use. Local users can download it with setup/download-cache.sh.
#
# Usage:
#   ./cranelift/isle/veri/verify.sh [MODE] [CONFIG ...]
#
# Modes:
#   cache-only     Verify purely from the local cache, in read-only, enforcing
#                  mode. Fails on any cache miss and never invokes an SMT
#                  solver. Use this to validate that a previously generated
#                  cache fully covers the current backends.
#
#   rebuild-cache  Regenerate the cache: read from the existing cache, write
#                  only the entries actually used into a fresh directory, then
#                  swap it in. Unused entries are dropped (garbage collected).
#                  Cache misses are computed by invoking the SMT solver, so
#                  this requires z3 and/or cvc5 to be installed.
#
#   (no argument)  Local development: use and update the cache in-place.
#                  Serves cached results where possible and invokes the solver
#                  on misses, writing new entries back to the cache.
#
# CONFIG names (without the .args suffix) optionally select the
# configurations to run from cranelift/isle/veri/configs/; the default is
# CONFIGS below.

set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

MODE="${1:-local}"
shift || true

CACHE_DIR="cranelift/isle/veri/cache"
# The default set of configurations to verify.
CONFIGS=(
    cranelift/isle/veri/configs/aarch64.args
    cranelift/isle/veri/configs/opt.args
    cranelift/isle/veri/configs/x64-iadd-base-case.args
)
# CI passes its own list (see the isle_veri_full_check job in
# .github/workflows/main.yml).
if [ $# -gt 0 ]; then
    CONFIGS=()
    for name in "$@"; do
        CONFIGS+=("cranelift/isle/veri/configs/${name}.args")
    done
fi

# Run the verifier for every configuration, forwarding the given cache flags.
run_all() {
    for config in "${CONFIGS[@]}"; do
        echo "=== veri: $config ==="
        cargo run -p cranelift-isle-veri --release --bin veri -- --config "$config" "$@"
    done
}

case "$MODE" in
cache-only)
    echo "=== Verifying from cache (read-only, enforcing; no solver) ==="
    if [ ! -d "$CACHE_DIR" ]; then
        echo "ERROR: cache directory does not exist: $CACHE_DIR" >&2
        echo "Generate it first (requires z3 and/or cvc5):" >&2
        echo "  ./cranelift/isle/veri/verify.sh rebuild-cache" >&2
        exit 1
    fi
    if ! run_all --cache-source-dir "$CACHE_DIR" --cache-mode read-only-enforcing; then
        cat >&2 <<EOF

Verification failed (cache miss or verification error).

A cache miss means the ISLE source in the backends changed such that
there are new SMT queries not present in the cache. Regenerate it
(requires z3 and/or cvc5):

  ./cranelift/isle/veri/verify.sh rebuild-cache

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
    echo "=== Cache rebuilt at $CACHE_DIR ==="
    ;;

local | "")
    echo "=== Verifying with local cache (read-write; in-place) ==="
    run_all --cache-source-dir "$CACHE_DIR" \
        --cache-dest-dir "$CACHE_DIR" \
        --cache-mode read-write
    echo "=== Done ==="
    ;;

*)
    echo "usage: $0 [cache-only | rebuild-cache] [CONFIG ...]" >&2
    echo "       (no argument runs a local, in-place read-write verification)" >&2
    echo "       (CONFIG names are .args files in cranelift/isle/veri/configs/)" >&2
    exit 1
    ;;
esac
