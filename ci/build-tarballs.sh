#!/bin/bash

# A small shell script invoked from CI on the final Linux builder which actually
# assembles the release artifacts for a particular platform. This will take the
# binary artifacts of previous builders and create associated tarballs to
# publish to GitHub.
#
# The first argument of this is the "platform" name to put into the tarball, and
# the second argument is the name of the github actions platform which is where
# we source binaries from. The final third argument is ".exe" on Windows to
# handle executable extensions right.
#
# Usage: build-tarballs.sh PLATFORM [.exe]

# where PLATFORM is e.g. x86_64-linux, aarch64-linux, ...

set -ex

platform=$1
exe=$2

rm -rf tmp
mkdir tmp
mkdir -p dist

mktarball() {
  dir=$1
  if [ "$exe" = "" ]; then
    tar cJf dist/$dir.tar.xz -C tmp $dir
  else
    (cd tmp && zip -r ../dist/$dir.zip $dir)
  fi
}

# Create the main tarball of binaries
bin_pkgname=wasmtime-$TAG-$platform
mkdir tmp/$bin_pkgname
cp LICENSE README.md tmp/$bin_pkgname
mv bins-$platform/wasmtime$exe tmp/$bin_pkgname
chmod +x tmp/$bin_pkgname/wasmtime$exe
mktarball $bin_pkgname

if [ -f bins-$platform/installer.msi ]; then
  mv bins-$platform/installer.msi dist/$bin_pkgname.msi
fi

# Create tarball of API libraries
api_pkgname=wasmtime-$TAG-$platform-c-api
mkdir tmp/$api_pkgname
mkdir tmp/$api_pkgname/lib
mkdir tmp/$api_pkgname/include
cp LICENSE README.md tmp/$api_pkgname
mv bins-$platform/* tmp/$api_pkgname/lib
cp crates/c-api/wasm-c-api/include/wasm.h tmp/$api_pkgname/include
cp crates/c-api/include/{wasmtime,wasi}.h tmp/$api_pkgname/include
mktarball $api_pkgname
