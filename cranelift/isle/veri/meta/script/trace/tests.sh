#!/bin/bash

set -exuo pipefail

tests_directory="$1"

# Copy to tests_directory directory.
#
# Exclude:
# simd*: All SIMD is out of scope.
# table-sub, table_copy, table_init, bulk, memory_fill, memory_copy:
#   bulk memory ops extension
# binary: limited to binary parsing tests
# binary-leb128: Uses `trunc_sat_*` instructions, and seems to be mostly about
#   parsing the binary format anyway.
shopt -s extglob
cp tests/spec_testsuite/!(simd*|table-sub|table_copy|table_init|bulk|memory_fill|memory_copy|binary|binary-leb128).wast "${tests_directory}"

# Remove `i(32|64).extend(8|16|32)_s` instructions and the tests that use
# them, which are part of the sign-extending operators extension.
sed -i.bak '/extend[0-9][0-9]*_s/d' "${tests_directory}/i32.wast"
sed -i.bak '/extend[0-9][0-9]*_s/d' "${tests_directory}/i64.wast"

# Same with `trunc_sat_f(32|64)_(s|u)` instructions, which are part of the
# non-trapping float-to-int extension.
sed -i.bak '/trunc_sat_/d' "${tests_directory}/conversions.wast"

# Clean.
rm "${tests_directory}"/*.bak
