#!/bin/bash
set -euo pipefail
cd $(dirname "$0")
topdir="$(pwd)"

# All the cretonne-* crates have the same version number
version="0.8.0"

# Update all of the Cargo.toml files.
#
# The main Cargo.toml in the top-level directory is the cretonne-tools crate which we don't publish.
echo "Updating crate versions to $version"
for crate in . lib/*; do
    # Update the version number of this crate to $version.
    sed -i.bk -e "s/^version = .*/version = \"$version\"/" "$crate/Cargo.toml"
    # Update the required version number of any cretonne* dependencies.
    sed -i.bk -e "/^cretonne/s/version = \"[^\"]*\"/version = \"$version\"/" "$crate/Cargo.toml"
done

# Update our local Cargo.lock (not checked in).
cargo update
./test-all.sh

# Commands needed to publish.
#
# Note that libraries need to be published in topological order.

echo git commit -a -m "\"Bump version to $version"\"
echo git push
for crate in entity codegen frontend native reader wasm module simplejit faerie umbrella ; do
    if [ "$crate" == "umbrella" ]; then
        dir="cretonne"
    else
        dir="$crate"
    fi

    echo cargo publish --manifest-path "lib/$crate/Cargo.toml"
done
echo
echo Then, go to https://github.com/cretonne/cretonne/releases/ and define a new release.
