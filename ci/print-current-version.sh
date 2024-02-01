#!/bin/sh
grep '^version =' Cargo.toml | head -n 1 | sed 's/.*"\(.*\)"/\1/'
