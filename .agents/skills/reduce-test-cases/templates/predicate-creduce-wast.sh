#!/usr/bin/env bash
#
# PREDICATE SKELETON for `creduce` on a `.wast` script.
#
# `wasm-tools shrink` reduces ONE module; a `.wast` is a whole script (many
# modules + assertions), so use creduce on the text.
#
#   cp input.wast predicate-creduce-wast.sh /tmp/reduce && cd /tmp/reduce
#   creduce ./predicate-creduce-wast.sh input.wast
#
# creduce runs this from a FRESH TEMP DIR with the candidate by BASENAME:
# reference the candidate by basename, tools by ABSOLUTE path, exit 0 == "still
# reproduces".

CAND=input.wast

# EDIT: absolute path to the engine you built.
#   echo "$PWD/target/release/wasmtime"
WASMTIME=/ABS/PATH/TO/target/release/wasmtime

# `wasmtime wast` runs the script and its assertions. Feature flags come from -W
# on the CLI, NOT from the `;;! feature = true` header (that header is only read
# by `cargo test --test wast`). Add any your case needs, e.g. -W gc,function-references.
out=$("$WASMTIME" wast "$CAND" 2>&1)
st=$?

# EDIT: pin YOUR exact symptom. Examples:
#   unexpected trap / failure:  echo "$out" | grep -q 'integer divide by zero'
#   any failure at all:         [ $st -ne 0 ]
echo "$out" | grep -q 'integer divide by zero'

# NOTE: creduce will happily DELETE the `;;! ...` header (the CLI ignores it, so
# removing it doesn't change this predicate). If the reduced case is destined for
# tests/ (run by `cargo test --test wast`), re-add the header afterward, or the
# harness won't enable the right features.
