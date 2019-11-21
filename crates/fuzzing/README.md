# Fuzzing Infrastructure for Wasmtime

This crate provides test case generators and oracles for use with fuzzing.

These generators and oracles are generally independent of the fuzzing engine
that might be using them and driving the whole fuzzing process (e.g. libFuzzer
or AFL). As such, this crate does *not* contain any actual fuzz targets
itself. Those are generally just a couple lines of glue code that plug raw input
from (for example) `libFuzzer` into a generator, and then run one or more
oracles on the generated test case.

If you're looking for the actual fuzz target definitions we currently have, they
live in `wasmtime/fuzz/fuzz_targets/*` and are driven by `cargo fuzz` and
`libFuzzer`.
