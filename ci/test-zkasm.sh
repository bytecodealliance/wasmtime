#!/usr/bin/env bash

set -o pipefail
set -eux

npm install --prefix tests/zkasm
TEST_PATH=../../${1:-"cranelift/zkasm_data/generated"}
npm test --prefix tests/zkasm $TEST_PATH
