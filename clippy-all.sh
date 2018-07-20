#!/bin/bash
set -euo pipefail

# Check all sources with clippy.
# In the clif-util crate (root dir) clippy will only work with nightly cargo -
# there is a bug where it will reject the commands passed to it by cargo 0.25.0
exec cargo +nightly clippy --all
