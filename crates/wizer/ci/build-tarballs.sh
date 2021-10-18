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
bin_pkgname=wizer-$TAG-$platform
mkdir tmp/$bin_pkgname
cp README.md tmp/$bin_pkgname
mv bins-$platform/wizer$exe tmp/$bin_pkgname
chmod +x tmp/$bin_pkgname/wizer$exe
mktarball $bin_pkgname
