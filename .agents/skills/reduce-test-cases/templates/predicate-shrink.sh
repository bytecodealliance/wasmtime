#!/usr/bin/env bash
#
# PREDICATE SKELETON for `wasm-tools shrink` (wasm / wat).
#
#   wasm-tools shrink ./this-script.sh input.wasm -o reduced.wasm
#
# shrink calls this script with the candidate module as $1 and keeps shrinking
# as long as the script exits 0. So: exit 0 == "still reproduces my bug".
#
# COPY this file, then edit the marked spots for YOUR bug. Test it on the
# ORIGINAL module first — if it doesn't exit 0 there, shrink has nothing to do.
#
# Do NOT add `set -e`: a trapping module makes wasmtime exit non-zero, and under
# `set -e` the `out=$(...)` line would abort the script before your grep runs.

CAND="${1:?shrink passes the candidate as \$1}"

# EDIT: absolute path to the engine you built. Absolute because reducers run
# from other directories. Get it with:  echo "$PWD/target/release/wasmtime"
WASMTIME=/ABS/PATH/TO/target/release/wasmtime

# OPTIONAL — keep the reduction honest: require the instruction/feature that
# triggers your bug to still be present, so shrink can't "fix" it by deleting
# it. Uncomment and adapt:
# wasm-tools print "$CAND" | grep -q 'i32.div_s' || exit 1

# EDIT: the symptom. Pin it as tightly as you can — a loose predicate lets
# shrink converge on a DIFFERENT, spec-legal module that fails some other way.
# Pick/adapt ONE shape:
#
#   trap message (this example):
out=$("$WASMTIME" run --invoke _start "$CAND" 2>&1)
echo "$out" | grep -q 'integer divide by zero'
#
#   hard crash (segfault/abort => status >= 128; a clean wasm trap is lower):
#     "$WASMTIME" run "$CAND" >/dev/null 2>&1; [ $? -ge 128 ]
#
#   miscompile (two opt levels disagree): use templates/predicate-differential.sh
#   instead — a differential needs exit-0 + non-empty guards or the reducer
#   converges on a merely-broken program. The naive `[ "$a" != "$b" ]` is unsound.
#
# AFTER shrinking, eyeball the result — `wasm-tools print reduced.wasm` — and
# re-confirm it's still YOUR bug, not a lookalike.
