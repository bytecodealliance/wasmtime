#!/usr/bin/env bash
#
# COMMAND SKELETON for Binaryen's `wasm-reduce` (wasm).
#
#   wasm-reduce input.wasm --command 'bash ./this-script.sh' \
#     --test t.wasm --working reduced.wasm
#   # add -f if wasm-reduce refuses to start (see note 3 below)
#
# wasm-reduce works DIFFERENTLY from creduce / wasm-tools shrink:
#
#   1. It writes each candidate to the FIXED --test file (t.wasm here), then
#      runs this command. This command must read that fixed path, NOT $1.
#   2. A candidate is kept iff this command's STDOUT *and* EXIT CODE match what
#      they were on the original. It preserves behavior, it does not look for
#      "exit 0". So collapse "my bug reproduces" down to ONE canonical line of
#      stdout (below), and let everything else print something different.
#   3. Before reducing, wasm-reduce round-trips the module through Binaryen's
#      own parser/writer as a sanity check. If Binaryen can't handle a wasm
#      proposal in your module it prints "failed to read and write the binary"
#      and stops — pass -f to force past it (reduction still runs; it just skips
#      Binaryen's structural passes and relies on generic shrinking). If even -f
#      gets nowhere, fall back to disassemble + creduce (see predicate-creduce-wat.sh).

# Must match the --test filename above.
TEST=t.wasm

# EDIT: absolute path to the engine you built.
#   echo "$PWD/target/release/wasmtime"
WASMTIME=/ABS/PATH/TO/target/release/wasmtime

out=$("$WASMTIME" run --invoke _start "$TEST" 2>&1)

# EDIT: the symptom. Print the SAME canonical marker for every candidate that
# reproduces, and anything else otherwise. (Exit code is also preserved, so
# keep it stable too — here we always exit 0 and distinguish via stdout.)
if echo "$out" | grep -q 'integer divide by zero'; then
  echo REPRODUCES
else
  echo no
fi
