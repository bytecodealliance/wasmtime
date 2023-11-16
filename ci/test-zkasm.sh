#!/usr/bin/env bash

# Should be runned from wasmtime/
#
# If you run in preinstalled mode, assumes that you have https://github.com/0xPolygonHermez/zkevm-rom
# in same directory as wasmtime.

set -o pipefail
set -eux

# Flags and default modes
ALL_FILES=false

# Parse flags
while [[ "$#" -gt 0 ]]; do
    case $1 in
        --all) ALL_FILES=true; shift ;;
        --help)
            echo "Usage: $0 [OPTIONS] [filename.zkasm]"
            echo "Options:"
            echo "  --all                           Test all zkasm files"
            echo "  --help                          Show this message"
            exit 0
            ;;
        *) break ;;
    esac
done

if [ "$ALL_FILES" = false ] && [ -z "$1" ]; then
    echo "Please provide a filename or use the --all flag to test all files."
    exit 1
fi

BASE_DIR="."

NODE_CMD="node deps/zkevm-proverjs/test/zkasmtest.js --rows 2**18"

if [ "$ALL_FILES" = false ]; then
    $NODE_CMD "$BASE_DIR/$1"
    exit 0
fi

FAIL_PREFIX="_should_fail_"
all_passed=true

for file in "$BASE_DIR/cranelift/zkasm_data/generated"/*; do
  filename=$(basename -- "$file")
  # it seems like zkasmtest sets 1 if smth goes wrong but don't set 0
  # if everything is OK
  exit_code=0

  if [[ $filename == $FAIL_PREFIX* ]]; then
    # If the file name starts with "_should_fail_", we should expect a non-zero exit code
    $NODE_CMD "$file" > /dev/null 2>&1 || exit_code=$?
    if [[ $exit_code -ne 0 ]]; then
      echo -e "\033[0;32m    --> fail\033[0m $BASE_DIR/cranelift/zkasm_data/generated/$filename"
    else
      echo -e "\033[0;31m    --> pass\033[0m $BASE_DIR/cranelift/zkasm_data/generated/$filename"
      all_passed=false
    fi
  else
    $NODE_CMD "$file" > /dev/null 2>&1 || exit_code=$?
    if [[ $exit_code -ne 0 ]]; then
      echo -e "\033[0;31m    --> fail\033[0m $BASE_DIR/cranelift/zkasm_data/generated/$filename"
      all_passed=false
    else
      echo -e "\033[0;32m    --> pass\033[0m $BASE_DIR/cranelift/zkasm_data/generated/$filename"
    fi
  fi
done

# Exit with 0 if all tests passed, 1 otherwise
if $all_passed; then
  exit 0
else
  exit 1
fi
