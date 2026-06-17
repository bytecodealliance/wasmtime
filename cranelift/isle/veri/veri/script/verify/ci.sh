#!/usr/bin/env bash

set -euo pipefail

RUST_LOG=info ./script/veri.sh "$@" -- \
    --num-threads 0 \
    --results-to-log-dir \
    --ignore-solver-tags \
    --filter include:first-rule-named \
    --filter exclude:tag:vector \
    --filter exclude:tag:slow \
    --filter exclude:tag:i128 \
    --filter exclude:tag:atomics \
    --filter exclude:tag:narrowfloat \
    --filter exclude:tag:amode_const \
    ;
