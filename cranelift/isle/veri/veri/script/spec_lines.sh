#!/usr/bin/env bash

set -exuo pipefail

RUST_LOG=isle_spec_lines=trace cargo run --bin spec_lines -- \
    --codegen-crate-dir ../../../codegen/ \
    --work-dir /tmp \
    --name aarch64 \
    2>&1 | grep SPEC_LINES
