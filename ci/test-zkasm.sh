#!/usr/bin/env bash

set -o pipefail
set -eux

# NB: This might have false-positives locally, but it's worth it for iteration speed.
# If you ever run into a situation when modules are out of date, run this command manually.
if [ ! -d "tests/zkasm/node_modules" ]; then
	npm install --prefix tests/zkasm
fi
TEST_PATH=../../${1:-"cranelift/zkasm_data/generated"}
npm test --prefix tests/zkasm $TEST_PATH
