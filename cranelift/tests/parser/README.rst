Parser tests
============

This directory contains test cases for the Cretonne IL parser.

Each test case consists of a `foo.cton` input file and a `foo.ref` reference
output file. Each input file is run through the `cton-util cat` command, and the
output is compared against the reference file. If the two are identical, the
test passes.
