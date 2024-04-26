# `cargo fuzz` Targets for Wasmtime

This crate defines various [libFuzzer](https://www.llvm.org/docs/LibFuzzer.html)
fuzzing targets for Wasmtime, which can be run via [`cargo
fuzz`](https://rust-fuzz.github.io/book/cargo-fuzz.html).

These fuzz targets just glue together pre-defined test case generators with
oracles and pass libFuzzer-provided inputs to them. The test case generators and
oracles themselves are independent from the fuzzing engine that is driving the
fuzzing process and are defined in `wasmtime/crates/fuzzing`.

## Safety warning

Fuzzers exist to generate random garbage and then try running it. **You
should not trust these fuzz targets**: they could in theory try to read
or write files on your disk, send your private data to reporters, or do
anything else. If they succeed at doing something malicious, they are
doing a great job at identifying dangerous bugs and we're proud of them.

In addition, some of these fuzz targets use other libraries, such as to
test that our implementation matches other WebAssembly runtimes. **We
have not reviewed those runtimes or libraries** for safety, security,
correctness, supply-chain attacks, or any other properties. Software
used only during fuzzing is not subject to [our usual `cargo vet`
requirements][vet-docs].

[vet-docs]: https://docs.wasmtime.dev/contributing-coding-guidelines.html#dependencies-of-wasmtime

Paragraphs 7 and 8 of the license which this work is distributed to you
under are especially important here: **We disclaim all warranties and
liability** if running some fuzz target causes you any harm.

Therefore, **if you are at all concerned about the safety of your
computer**, then you should either not run these fuzz targets, or only
run them in a sandbox with sufficient isolation for your threat model.

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
* `cranelift-icache`: Generate a Cranelift function A, applies a small mutation
  to its source, yielding a function A', and checks that A compiled +
  incremental compilation generates the same machine code as if A' was compiled
  from scratch.
* `differential`: Generate a Wasm module, evaluate each exported function
  with random inputs, and check that Wasmtime returns the same results as a
  choice of another engine: the Wasm spec interpreter (see the
  `wasm-spec-interpreter` crate), the `wasmi` interpreter, V8 (through the `v8`
  crate), or Wasmtime itself run with a different configuration.
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

## Target specific options

### `cranelift-fuzzgen`

Fuzzgen supports passing the `FUZZGEN_ALLOWED_OPS` environment variable, which when available restricts the instructions that it will generate.

Running `FUZZGEN_ALLOWED_OPS=ineg,ishl cargo fuzz run cranelift-fuzzgen` will run fuzzgen but only generate `ineg` or `ishl` opcodes.

### `cranelift-icache`

The icache target also uses the fuzzgen library, thus also supports the `FUZZGEN_ALLOWED_OPS` enviornment variable as described in the `cranelift-fuzzgen` section above.

