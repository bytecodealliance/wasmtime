#!/bin/bash

# Exit immediately on errors.
set -e

# Run from the src/tools directory which includes all our crates.
cd $(dirname "$0")/tools

PKGS="-p cretonne -p cretonne-reader -p cretonne-tools"
cargo build $PKGS
cargo doc  $PKGS
cargo test $PKGS
