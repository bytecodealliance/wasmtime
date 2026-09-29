#!/usr/bin/env bash
#
# PREDICATE SKELETON for `creduce` on a disassembled `.wat` (wasm fallback).
#
# Use this when neither `wasm-tools shrink` nor `wasm-reduce` can handle your
# module (e.g. a wasm proposal neither tool fully supports). creduce reduces the
# TEXT and never re-encodes the wasm, so only the ENGINE has to understand the
# proposal — the reducer stays out of the way.
#
#   wasm-tools print input.wasm -o case.wat      # disassemble
#   cp case.wat predicate-creduce-wat.sh /tmp/reduce && cd /tmp/reduce
#   creduce ./predicate-creduce-wat.sh case.wat   # reduces case.wat in place
#
# creduce copies the candidate into a FRESH TEMP DIR and runs this test there,
# referring to the file by BASENAME. So: reference the candidate by basename
# (no path), reference every tool by an ABSOLUTE path, and exit 0 == "still
# reproduces". `wasmtime` runs a `.wat` directly, so no reassembly is needed.

# Candidate basename (matches the filename you pass to creduce). No directory.
CAND=case.wat

# EDIT: absolute path to the engine you built.
#   echo "$PWD/target/release/wasmtime"
WASMTIME=/ABS/PATH/TO/target/release/wasmtime

out=$("$WASMTIME" run --invoke _start "$CAND" 2>&1)

# EDIT: pin YOUR exact symptom. A parse error on a broken intermediate .wat will
# NOT contain this string, so creduce correctly rejects invalid candidates.
echo "$out" | grep -q 'integer divide by zero'

# After creduce finishes, eyeball case.wat and re-confirm it's still your bug.
