#!/bin/bash
#
# Driver for VeriISLE verification with the SMT query cache.
#
# The cache lives in an S3 bucket, named by ISLE_VERI_CACHE_BUCKET, as a base
# snapshot plus per-run deltas (see cranelift/isle/veri/caching/src/cache.rs).
# Requests are signed with AWS_ACCESS_KEY_ID / AWS_SECRET_ACCESS_KEY (and
# optionally AWS_SESSION_TOKEN) if set, and are anonymous otherwise, which
# suffices to read a public bucket. Writing requires credentials.
#
# Usage:
#   ./cranelift/isle/veri/verify.sh [MODE] [CONFIG ...]
#
# Modes:
#   cache-only     Verify purely from the cache, in read-only, enforcing
#                  mode. Fails on any cache miss and never invokes an SMT
#                  solver. Use this to validate that the cache fully covers
#                  the current backends.
#
#   read-write     Serve cached results where possible, invoke the solver on
#                  misses, and upload the new results as a delta at the end
#                  of each configuration's run. Then, if enough deltas have
#                  accumulated, compact the cache, dropping entries none of
#                  the configurations used. Requires credentials and z3
#                  and/or cvc5.
#
#   read-only      Serve cached results where possible and invoke the solver
#   (or no         on misses, without writing anything back. This is the
#   argument)      mode for local development.
#
# CONFIG names (without the .args suffix) optionally select the
# configurations to run from cranelift/isle/veri/configs/; the default is
# CONFIGS below.

set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

MODE="${1:-read-only}"
shift || true

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
# The bucket is passed through the environment (ISLE_VERI_CACHE_BUCKET).
run_all() {
    for config in "${CONFIGS[@]}"; do
        echo "=== veri: $config ==="
        cargo run -p cranelift-isle-veri --release --bin veri -- --config "$config" "$@"
    done
}

if [ -z "${ISLE_VERI_CACHE_BUCKET:-}" ]; then
    echo "ERROR: set ISLE_VERI_CACHE_BUCKET to the S3 bucket holding the cache" >&2
    exit 1
fi

case "$MODE" in
cache-only)
    echo "=== Verifying from cache (read-only, enforcing; no solver) ==="
    if ! run_all --cache-mode read-only-enforcing; then
        cat >&2 <<EOF

Verification failed (cache miss or verification error).

A cache miss means the ISLE source in the backends changed such that
there are new SMT queries not present in the cache. Populate it
(requires credentials and z3 and/or cvc5):

  ./cranelift/isle/veri/verify.sh read-write

EOF
        exit 1
    fi
    echo "=== Cache verification passed ==="
    ;;

read-write)
    echo "=== Verifying with cache (read-write; uploads new results) ==="
    # Each configuration's run records the cache keys it used in a shared
    # file. Once all have succeeded, compaction (if enough deltas have
    # accumulated) keeps only the entries listed there.
    used_keys=$(mktemp)
    trap 'rm -f "$used_keys"' EXIT
    run_all --cache-mode read-write --cache-used-keys-file "$used_keys"
    echo "=== Compacting cache if needed ==="
    cargo run -p cranelift-isle-veri --release --bin veri-cache -- \
        compact --used-keys-file "$used_keys"
    echo "=== Done ==="
    ;;

read-only | "")
    echo "=== Verifying with cache (read-only) ==="
    run_all --cache-mode read-only
    echo "=== Done ==="
    ;;

*)
    echo "usage: $0 [read-only | read-write | cache-only] [CONFIG ...]" >&2
    echo "       (CONFIG names are .args files in cranelift/isle/veri/configs/)" >&2
    exit 1
    ;;
esac
