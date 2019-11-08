#!/bin/bash
set -euo pipefail

# This is a trivial wrapper around cargo which just forwards its arguments
# to cargo, and then sleeps for a few seconds, to allow for exteral services
# to update their indices.
# https://internals.rust-lang.org/t/changes-to-how-crates-io-handles-index-updates/9608

cargo "$@"
sleep 10
