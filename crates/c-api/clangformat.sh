#!/usr/bin/env bash

set -eu

cd $(dirname $0)/../..

git ls-files '*.h' '*.c' '*.cpp' '*.hh' '*.cc' | \
    grep -v wasmtime-platform.h | \
    grep -v wasm.h | \
    xargs clang-format-18 -i
