#!/bin/bash

# Script to merge the outputs of a run on github actions to github releases.
# This is invoked from `.github/workflows/publish-artifacts.yml`. All previous
# artifacts from builds are located in `bins-*` folders. The main purpose of
# this script is to take the "min" build and merge it into the "normal" build to
# produce one final tarball. This means that the final artifacts will have both
# a normal and a min build in them for comparison and usage.

set -ex

# Prepare the upload folder and move all artifacts that aren't being merged into
# this folder, e.g. the MSI installer and adapter wasm files.
rm -rf dist
mkdir dist
mv -t dist bins-*/*.{msi,wasm}
mv wasmtime-platform-header/* dist

# Merge tarballs and zips by searching for `*-min` builds, unpacking the
# min/normal builds, into the same destination, and then repacking into a
# tarball.
#
# Note that for now xz compression is used for the final artifact to try to get
# small artifacts, but it's left at the default level since a lot of artifacts
# are processed here and turning it up to the max 9 compression might take
# quite awhile on CI for this one builder to process.
for min in bins-*-min/*.tar.*; do
 normal=${min/-min\//\/}
 filename=$(basename $normal)
 dir=${filename%.tar.gz}

 rm -rf tmp
 mkdir tmp
 tar xf $min -C tmp
 tar xf $normal -C tmp
 tar -cf - -C tmp $dir | xz -T0 > dist/$dir.tar.xz
 rm $min $normal
done

for min in bins-*-min/*.zip; do
  normal=${min/-min\//\/}
  filename=$(basename $normal)
  dir=${filename%.zip}

  rm -rf tmp
  mkdir tmp
  (cd tmp && unzip -o ../$min)
  (cd tmp && unzip -o ../$normal)
  (cd tmp && 7z a ../dist/$dir.zip $dir/)
  rm $min $normal
done

# Copy over remaining source tarball into the dist folder
mv -t dist bins-*/*.tar.*
