#!/usr/bin/env bash

# Excludes:
#
# - test-programs: just programs used in tests.
#
# - wasmtime-wasi-nn: mutually-exclusive features that aren't available for all
#   targets, needs its own CI job.
#
# - wasmtime-fuzzing: enabling all features brings in OCaml which is a pain to
#   configure for all targets, so it has its own CI job.
#
# - wasm-spec-interpreter: brings in OCaml which is a pain to configure for all
#   targets, tested as part of the wastime-fuzzing CI job.

cargo test \
      --workspace \
      --all-features \
      --exclude test-programs \
      --exclude wasmtime-wasi-nn \
      --exclude wasmtime-fuzzing \
      --exclude wasm-spec-interpreter \
      $@
