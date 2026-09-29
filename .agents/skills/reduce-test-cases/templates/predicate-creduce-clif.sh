#!/usr/bin/env bash
#
# PREDICATE SKELETON for `creduce` on a Cranelift `.clif` file.
#
# Use creduce for ANY clif bug that isn't a hard compile panic: miscompiles,
# wrong results, verifier errors, graceful codegen errors. (For an actual
# compile PANIC, prefer `clif-util bugpoint FILE TARGET` — it needs no
# predicate. See SKILL.md.)
#
#   cp input.clif predicate-creduce-clif.sh /tmp/reduce && cd /tmp/reduce
#   creduce ./predicate-creduce-clif.sh input.clif
#
# creduce runs this from a FRESH TEMP DIR with the candidate by BASENAME:
# reference the candidate by basename, tools by ABSOLUTE path, exit 0 == "still
# reproduces".

CAND=input.clif

# EDIT: absolute path to the tool you built.
#   echo "$PWD/target/release/clif-util"
CLIF_UTIL=/ABS/PATH/TO/target/release/clif-util

# EDIT: pick the sub-command that exposes YOUR bug, and pin the symptom.
#
#   codegen error / crash text (target on the CLI, so header lines aren't needed):
out=$("$CLIF_UTIL" compile --target riscv64 "$CAND" 2>&1)
echo "$out" | grep -q 'should be implemented in ISLE'
#
#   in-file `; run:` / `; check:` expectations (honors the file's directives):
#     "$CLIF_UTIL" test "$CAND" >/dev/null 2>&1; [ $? -ne 0 ]   # interesting == FAILS
#
#   miscompile via differential execution — first edit the file's expected value
#   so a CORRECT engine passes and only the buggy path fails, then:
#     out=$("$CLIF_UTIL" interpret "$CAND" 2>&1); echo "$out" | grep -q 'mismatch'
#
# NOTE: creduce deletes the `test ... / set ... / target ...` header lines (your
# predicate passes --target on the CLI, so they're not needed to reproduce). The
# reduced file therefore won't run under `clif-util test` unmodified — re-add the
# headers if you want a self-contained filetest. Eyeball with `clif-util cat`.
