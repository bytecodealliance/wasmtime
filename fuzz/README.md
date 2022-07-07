# `cargo fuzz` Targets for Wasmtime

This crate defines various [libFuzzer](https://www.llvm.org/docs/LibFuzzer.html)
fuzzing targets for Wasmtime, which can be run via [`cargo
fuzz`](https://rust-fuzz.github.io/book/cargo-fuzz.html).

These fuzz targets just glue together pre-defined test case generators with
oracles and pass libFuzzer-provided inputs to them. The test case generators and
oracles themselves are independent from the fuzzing engine that is driving the
fuzzing process and are defined in `wasmtime/crates/fuzzing`.

## Example

To start fuzzing run the following command, where `$MY_FUZZ_TARGET` is one of
the [available fuzz targets](#available-fuzz-targets):

```shell
cargo fuzz run $MY_FUZZ_TARGET
```

## Available Fuzz Targets

At the time of writing, we have the following fuzz targets:

* `api_calls`: stress the Wasmtime API by executing sequences of API calls; only
  the subset of the API is currently supported.
* `compile`: Attempt to compile libFuzzer's raw input bytes with Wasmtime.
* `compile-maybe-invalid`: Attempt to compile a wasm-smith-generated Wasm module
  with code sequences that may be invalid.
* `cranelift-fuzzgen`: Generate a Cranelift function and check that it returns
  the same results when compiled to the host and when using the Cranelift
  interpreter; only a subset of Cranelift IR is currently supported.
* `differential`: Generate a Wasm module and check that Wasmtime returns
  the same results when run with two different configurations.
* `differential_spec`: Generate a Wasm module and check that Wasmtime returns
  the same results as the Wasm spec interpreter (see the `wasm-spec-interpreter`
  crate).
* `differential_v8`: Generate a Wasm module and check that Wasmtime returns
  the same results as V8.
* `differential_wasmi`: Generate a Wasm module and check that Wasmtime returns
  the same results as the `wasmi` interpreter.
* `instantiate`: Generate a Wasm module and Wasmtime configuration and attempt
  to compile and instantiate with them.
* `instantiate-many`: Generate many Wasm modules and attempt to compile and
  instantiate them concurrently.
* `spectests`: Pick a random spec test and run it with a generated
  configuration.
* `table_ops`: Generate a sequence of `externref` table operations and run them
  in a GC environment.

The canonical list of fuzz targets is the `.rs` files in the `fuzz_targets`
directory:

```shell
ls wasmtime/fuzz/fuzz_targets/
```

## Corpora

While you *can* start from scratch, libFuzzer will work better if it is given a
[corpus](https://www.llvm.org/docs/LibFuzzer.html#corpus) of seed inputs to kick
start the fuzzing process. We maintain a corpus for each of these fuzz targets
in [a dedicated repo on
github](https://github.com/bytecodealliance/wasmtime-libfuzzer-corpus).

You can use our corpora by cloning it and placing it at `wasmtime/fuzz/corpus`:

```shell
git clone \
    https://github.com/bytecodealliance/wasmtime-libfuzzer-corpus.git \
    wasmtime/fuzz/corpus
```

## Reproducing a Fuzz Bug

When investigating a fuzz bug (especially one found by OSS-Fuzz), use the
following steps to reproduce it locally:

1. Download the test case (either the "Minimized Testcase" or "Unminimized
   Testcase" from OSS-Fuzz will do).
2. Run the test case in the correct fuzz target:
    ```shell
    cargo +nightly fuzz run <target> <test case>
    ```
    If all goes well, the bug should reproduce and libFuzzer will dump the
    failure stack trace to stdout
3. For more debugging information, run the command above with `RUST_LOG=debug`
   to print the configuration and WebAssembly input used by the test case (see
   uses of  `log_wasm` in the `wasmtime-fuzzing` crate).

