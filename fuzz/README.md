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

* `compile`: Attempt to compile libFuzzer's raw input bytes with Wasmtime.
* `instantiate`: Attempt to compile and instantiate libFuzzer's raw input bytes
  with Wasmtime.
* `instantiate_translated`: Pass libFuzzer's input bytes to `wasm-opt -ttf` to
  generate a random, valid Wasm module, and then attempt to instantiate it.

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
