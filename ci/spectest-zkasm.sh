#!/usr/bin/env bash

set -o pipefail
set -eux

# NB: This might have false-positives locally, but it's worth it for iteration speed.
# If you ever run into a situation when modules are out of date, run this command manually.
if [ ! -d "tests/zkasm/node_modules" ]; then
	npm install --prefix tests/zkasm
fi
# Assert results of running tests commited
TEST_PATH=../../${1:-"cranelift/zkasm_data/spectest/i64/generated"}
# We don't expect all tests will pass so ignore if testing script exits with non zero code
(npm test --prefix tests/zkasm $TEST_PATH || true) | python3 ci/zkasm-result.py
