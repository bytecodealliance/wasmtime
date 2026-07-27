#!/usr/bin/env bash
#
# PREDICATE SKELETON for a MISCOMPILE — the input computes DIFFERENT results
# under two engine configs (two opt-levels, or two engines). "Interesting" ==
# they DISAGREE. This is the dominant shape for real miscompile reductions:
# there's no trap string or panic to grep, so you diff behavior instead.
#
# Written as a `wasm-tools shrink` predicate (candidate is $1, exit 0 ==
# reproduces). For the creduce variant see the note at the bottom.
#
#   wasm-tools shrink ./this-script.sh input.wasm -o reduced.wasm
#
# BEFORE you trust this, confirm the bug is DETERMINISTIC. Run each config on
# the ORIGINAL a few times: every run of a given config must give the SAME
# answer, and the two configs must DISAGREE. If a single config is flaky
# (uninitialized memory, NaN payload bits, a nondeterministic host call) a
# differential predicate is UNSOUND — it "reproduces" on noise and the reducer
# converges on garbage. Pin the flakiness down (or seed it) first.

CAND="${1:?shrink passes the candidate as \$1}"

# EDIT: absolute path.  echo "$PWD/target/release/wasmtime"
WASMTIME=/ABS/PATH/TO/target/release/wasmtime

# EDIT: the entry point, feature flags, and args your repro needs. Fuzzer repros
# almost always invoke a NAMED export with ARGUMENTS and enable a proposal with
# a -W flag (the fuzzers run the full proposal set), e.g.:
#     -Wwide-arithmetic --invoke f "$CAND" 1 -1
# opt-level=0 vs default (default == opt-level=2) is the usual disagreeing pair;
# swap in two different engines if that's your differential instead.
a=$("$WASMTIME" -O opt-level=0 --invoke f "$CAND" 2>/dev/null); ra=$?
b=$("$WASMTIME"                --invoke f "$CAND" 2>/dev/null); rb=$?

# Interesting == BOTH configs ran cleanly (exit 0) AND both produced output AND
# the outputs differ. The exit-0 and non-empty guards are ESSENTIAL, not
# decoration: without them a candidate that merely became INVALID (a parse
# error, empty output) or started TRAPPING would satisfy `a != b` trivially, and
# the reducer would happily converge on a broken program instead of a
# miscompile. This guarded form is exactly what the real reductions used.
[ $ra -eq 0 ] && [ $rb -eq 0 ] && [ -n "$a" ] && [ -n "$b" ] && [ "$a" != "$b" ]

# CREDUCE VARIANT: identical body, but reference the candidate by BASENAME
# (e.g. CAND=case.wat) instead of $1, run creduce from a fresh temp dir, and
# hand the .wat straight to wasmtime (no reassembly). The two runs, the four
# guards, and the determinism caveat are all the same.
#
# WAST VARIANT: make the reduced case SELF-CHECKING instead of diffing stdout —
# encode the correct answer as (assert_return ...) / (assert_trap ...) and let
# `wasmtime wast` at each opt-level pass or fail. Then "interesting" is
# "opt-level=0 passes but default fails":
#     "$WASMTIME" wast -O opt-level=0 "$CAND" >/dev/null 2>&1; r0=$?
#     "$WASMTIME" wast                 "$CAND" >/dev/null 2>&1; rd=$?
#     [ $r0 -eq 0 ] && [ $rd -ne 0 ]
