#!/usr/bin/env bash

# Should be runned from wasmtime/
#
# If you run in preinstalled mode, assumes that you have https://github.com/0xPolygonHermez/zkevm-rom
# in same directory as wasmtime.

set -o pipefail
set -eox

# Flags and default modes
PREINSTALLED=true
ALL_FILES=false

# Parse flags
while [[ "$#" -gt 0 ]]; do
    case $1 in
        --all) ALL_FILES=true; shift ;;
        --install-zkwasm) PREINSTALLED=false; shift ;;
        --help)
            echo "Usage: $0 [OPTIONS] [filename.zkasm]"
            echo "Options:"
            echo "  --all                           Test all zkasm files"
            echo "  --install-zkwasm                Temporarily install and use zkevm-rom"
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

BASE_DIR="../wasmtime"

if [ "$PREINSTALLED" = false ]; then
    echo "Cloning zkevm-rom into /tmp directory..."
    git clone https://github.com/0xPolygonHermez/zkevm-rom/ ./tmp/zkevm-rom > /dev/null 2>&1
    cd ./tmp/zkevm-rom
    npm install
    BASE_DIR="../.."
else
    cd ../zkevm-rom
fi

if [ "$ALL_FILES" = false ]; then
    node tools/run-tests-zkasm.js "$BASE_DIR/$1"
    exit 0
fi

FAIL_PREFIX="_should_fail_"
all_passed=true

for file in "$BASE_DIR/cranelift/zkasm_data/generated"/*; do
  filename=$(basename -- "$file")
  
  if [[ $filename == $FAIL_PREFIX* ]]; then
    # If the file name starts with "_should_fail_", we should expect a non-zero exit code
    node tools/run-tests-zkasm.js "$file" > /dev/null 2>&1 || exit_code=$?
    if [[ $exit_code -ne 0 ]]; then
      echo -e "\033[0;32m    --> fail\033[0m $BASE_DIR/cranelift/zkasm_data/generated/$filename"
    else
      echo -e "\033[0;31m    --> pass\033[0m $BASE_DIR/cranelift/zkasm_data/generated/$filename"
      echo "    --> fail $filename"
      all_passed=false
    fi
  else
    # For all other files, just run the node command and show the output
    if ! node tools/run-tests-zkasm.js "$file"; then
      all_passed=false
    fi
  fi
done

# Exit with 0 if all tests passed, 1 otherwise
if $all_passed; then
  exit 0
else
  exit 1
fi
