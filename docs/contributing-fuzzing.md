# Fuzzing

## Test Case Generators and Oracles

Test case generators and oracles live in the `wasmtime-fuzzing` crate, located
in the `crates/fuzzing` directory.

A *test case generator* takes raw, unstructured input from a fuzzer and
translates that into a test case. This might involve interpreting the raw input
as "DNA" or pre-determined choices through a decision tree and using it to
generate an in-memory data structure, or it might be a no-op where we interpret
the raw bytes as if they were Wasm.

An *oracle* takes a test case and determines whether we have a bug. For example,
one of the simplest oracles is to take a Wasm binary as an input test case,
validate and instantiate it, and (implicitly) check that no assertions failed or
segfaults happened. A more complicated oracle might compare the result of
executing a Wasm file with and without optimizations enabled, and make sure that
the two executions are observably identical.

Our test case generators and oracles strive to be fuzzer-agnostic: they can be
reused with libFuzzer or AFL or any other fuzzing engine or driver.

## libFuzzer and `cargo fuzz` Fuzz Targets

We combine a test case generator and one more more oracles into a *fuzz
target*. Because the target needs to pipe the raw input from a fuzzer into the
test case generator, it is specific to a particular fuzzer. This is generally
fine, since they're only a couple of lines of glue code.

Currently, all of our fuzz targets are written for
[libFuzzer](https://www.llvm.org/docs/LibFuzzer.html) and [`cargo
fuzz`](https://rust-fuzz.github.io/book/cargo-fuzz.html). They are defined in
the `fuzz` subdirectory.

See
[`fuzz/README.md`](https://github.com/bytecodealliance/wasmtime/blob/master/fuzz/README.md)
for details on how to run these fuzz targets and set up a corpus of seed inputs.
