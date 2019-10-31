#!/bin/bash

set -ex

platform=$1
src=$2
exe=$3

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
cp LICENSE README.md CACHE_CONFIGURATION.md tmp/$bin_pkgname
mv bins-$src/{wasmtime,wasm2obj}$exe tmp/$bin_pkgname
chmod +x tmp/$bin_pkgname/{wasmtime,wasm2obj}$exe
mktarball $bin_pkgname

if [ "$exe" = ".exe" ]; then
  mv bins-$src/installer.msi dist/$bin_pkgname.msi
fi

# Create tarball of API libraries
api_pkgname=wasmtime-$TAG-$platform-c-api
mkdir tmp/$api_pkgname
mkdir tmp/$api_pkgname/lib
mkdir tmp/$api_pkgname/include
cp LICENSE README.md tmp/$api_pkgname
mv bins-$src/* tmp/$api_pkgname/lib
cp wasmtime-api/c-examples/wasm-c-api/include/wasm.h tmp/$api_pkgname/include
mktarball $api_pkgname

# Move wheels to dist folder
mv wheels-$src/* dist
