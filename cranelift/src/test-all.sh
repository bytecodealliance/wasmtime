#!/bin/bash
cd $(dirname "$0")/tools
cargo test -p cretonne -p cretonne-reader -p cretonne-tools
cargo doc -p cretonne -p cretonne-reader -p cretonne-tools
