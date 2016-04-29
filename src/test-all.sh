#!/bin/bash
cd $(dirname "$0")/tools
cargo test -p cretonne -p ctonfile -p cretonne-tools
cargo doc -p cretonne -p ctonfile -p cretonne-tools
