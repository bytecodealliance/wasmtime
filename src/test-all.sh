#!/bin/bash
cd $(dirname "$0")/tools
PKGS="-p cretonne -p cretonne-reader -p cretonne-tools"
cargo build $PKGS
cargo doc  $PKGS
cargo test $PKGS
