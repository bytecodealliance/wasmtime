#!/usr/bin/env bash

set -euo pipefail

cargo run --bin status -- \
    --codegen-crate-dir ../../../codegen/ \
    --work-dir /tmp \
    "$@"
