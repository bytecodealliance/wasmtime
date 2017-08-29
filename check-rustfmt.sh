#!/bin/bash
#
# Usage: check-rustfmt.sh [--install]
#
# Check that the desired version of rustfmt is installed.
#
# Rustfmt is still immature enough that its formatting decisions can change
# between versions. This makes it difficult to enforce a certain style in a
# test script since not all developers will upgrade rustfmt at the same time.
# To work around this, we only verify formatting when a specific version of
# rustfmt is installed.
#
# Exits 0 if the right version of rustfmt is installed, 1 otherwise.
#
# With the --install option, also tries to install the right version.

# This version should always be bumped to the newest version available.
VERS="0.8.4"

if cargo install --list | grep -q "^rustfmt v$VERS"; then
    exit 0
fi

if [ "$1" != "--install" ]; then
    echo "********************************************************************"
    echo "*  Please install rustfmt v$VERS to verify formatting.             *"
    echo "*  If a newer version of rustfmt is available, update this script. *"
    echo "********************************************************************"
    echo "$0 --install"
    sleep 1
    exit 1
fi

echo "Installing rustfmt v$VERS."
cargo install --force --vers="$VERS" rustfmt
