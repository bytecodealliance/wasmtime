#!/usr/bin/env python3

# Excludes:
#
# - test-programs: just programs used in tests.
#
# - wasmtime-wasi-nn: mutually-exclusive features that aren't available for all
#   targets, needs its own CI job.
#
# - wasmtime-wasi-tls-nativetls: the openssl dependency does not play nice with
#   cross compilation. This crate is tested in a separate CI job.
#
# - wasmtime-fuzzing: enabling all features brings in OCaml which is a pain to
#   configure for all targets, so it has its own CI job.
#
# - wasm-spec-interpreter: brings in OCaml which is a pain to configure for all
#   targets, tested as part of the wastime-fuzzing CI job.
#
# - veri_engine: requires an SMT solver (z3)

import subprocess
import sys

args = ['cargo', 'test', '--workspace', '--all-features']
args.append('--exclude=test-programs')
args.append('--exclude=wasmtime-wasi-nn')
args.append('--exclude=wasmtime-wasi-tls-nativetls')
args.append('--exclude=wasmtime-fuzzing')
args.append('--exclude=wasm-spec-interpreter')
args.append('--exclude=veri_engine')
args.extend(sys.argv[1:])

result = subprocess.run(args)
sys.exit(result.returncode)
