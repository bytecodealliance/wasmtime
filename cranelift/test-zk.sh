#!/bin/bash

# ANSI escape codes for colors
RESET="\033[0m"
RED="\033[31m"
GREEN="\033[32m"

# Change to the desired directory
cd ../../zkevm-proverjs

# Default behavior: operate on one provided file
if [[ "$1" != "--all" && -n "$1" ]]; then
    zkasm_file="../wasmtime/cranelift/$1"
    if [[ -f "$zkasm_file" ]]; then
        echo "Processing file: $(basename $zkasm_file)"
        node test/zkasmtest.js "$zkasm_file"
    else
        echo -e "${RED}File $zkasm_file does not exist!${RESET}"
    fi
    exit 0
elif [[ "$1" == "--all" ]]; then
    # Checkout the training branch
    git checkout training 1> /dev/null 2>/dev/null

    # Initialize counters
    passed_count=0
    failed_count=0

    # Iterate over all .zkasm files in the specified directory
    for zkasm_file in ../wasmtime/cranelift/data/*.zkasm; do
        # Print only the name of the current file
        echo "Processing file: $(basename $zkasm_file)"

        # Execute the node command for each file and use grep and awk to process the output
        output=$(node test/zkasmtest.js "$zkasm_file" | grep -Eo "cntSteps: [0-9]+")

        # Check if output is empty or not
        if [[ -z "$output" ]]; then
            echo -e "${RED}ZKWASM ERROR${RESET}"
            ((failed_count++))
        else
            echo -e "${GREEN}OK. $(echo $output | awk -F': ' '{print $1 " = " $2}')${RESET}"
            ((passed_count++))
        fi
    done

    # Print the final counts with coloring
    echo -e "\n${GREEN}$passed_count files passed${RESET}"
    echo -e "${RED}$failed_count files failed${RESET}"
else
    echo "Usage:"
    echo "$0 <filename.zkasm>         - Process a specific file."
    echo "$0 --all                    - Process all .zkasm files."
fi
