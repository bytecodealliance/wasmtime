#!/usr/bin/env bash

# Runs Souper on the LHSes that were harvested by `clif-util
# souper-harvest`.
#
# This script takes two inputs:
#
# 1. The `souper-check` binary, and
# 2. The  directory of harvested  left-hand sides  (aka the `-o  $directory` you
#    specified to `clif-util souper-harvest`).
#
# For a left-hand side file `foo` that Souper successfully synthesized a
# right-hand side for, this script will write the whole optimization to a
# sibling file named `foo.result`.
#
# The left-hand sides are processed in smallest-to-largest order. This helps
# give you initial results more quickly, but does mean that progress will slow
# down as we encounter larger and larger left-hand sides.
#
# Usage:
#
#     run-souper.sh path/to/souper-check path/to/left-hand-sides

set -e

# Knobs for configuring how large of right-hand sides Souper should try to
# generate and how much time we give it to synthesize a result. Feel free to
# play with these!
MAX_INSTS=3
TIMEOUT=5s

# Run Souper on one left-hand side.
function run_one {
    local souper=$1
    local lhs=$2
    local rhs="$lhs".result

    if [[ -f "$rhs" ]]; then
        return
    fi

    local temp=$(mktemp)
    local cmd="taskset --cpu-list 0-3 $souper --infer-rhs -souper-enumerative-synthesis-max-instructions=$MAX_INSTS $lhs > $temp"

    set +e
    $(which timeout) --foreground --kill-after=1s $TIMEOUT bash -c "$cmd"
    local exit_code="$?"
    set -e

    case "$exit_code" in
        "0")
            # Success! Copy the RHS to its final destination.
            cp $lhs $rhs
            cat "$temp" >> "$rhs"
            ;;

        # SIGINT. Exit this whole script.
        "130")
            exit 1
            ;;

        # Timeout (regular).
        "124")
            return
            ;;

        # Timeout (with SIGKILL).
        "137")
            return
            ;;

        # Something else.
        *)
            exit 1
    esac

}

# Run Souper on all the left-hand sides.
function main {
    local souper=$1
    local lhs_dir=$2
    local lhs_count=$(ls -1 $lhs_dir | grep -v result | wc -l)

    echo "Processing $lhs_count left-hand sides."

    cd "$lhs_dir"

    local i=0;
    for lhs in $(ls -1S $lhs_dir); do
        # Ignore '.result' files.
        if $(echo "$lhs" | grep -q result); then
            continue;
        fi

        i=$(( $i + 1 ))
        if (( $i % 5 == 0 )); then
            local percent=$(( $i * 100 / $lhs_count ))
            echo "$i / $lhs_count ($percent%)"
        fi

        run_one "$souper" "$lhs"
    done

    echo "Done!"
}

# Kick everything off!
main $1 $2
