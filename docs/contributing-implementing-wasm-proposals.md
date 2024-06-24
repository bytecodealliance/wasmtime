# Implementing WebAssembly Proposals

## Adding New Support for a Wasm Proposal

The following checkboxes enumerate the steps required to add support for a new
WebAssembly proposal to Wasmtime. They can be completed over the course of
multiple pull requests.

* <input type="checkbox"/> Add support to the
  [`wasmparser`](https://github.com/bytecodealliance/wasm-tools/tree/main/crates/wasmparser)
  crate.

* <input type="checkbox"/> Add support to the
  [`wat`](https://github.com/bytecodealliance/wasm-tools/tree/main/crates/wat)
  and
  [`wast`](https://github.com/bytecodealliance/wasm-tools/tree/main/crates/wast)
  crates.

* <input type="checkbox"/> Add support to the
  [`wasmprinter`](https://github.com/bytecodealliance/wasm-tools/tree/main/crates/wasmprinter)
  crate.

* <input type="checkbox"/> Add support to the
  [`wasm-encoder`](https://github.com/bytecodealliance/wasm-tools/tree/main/crates/wasm-encoder)
  crate.

* <input type="checkbox"/> Add support to the
  [`wasm-smith`](https://github.com/bytecodealliance/wasm-tools/tree/main/crates/wasm-smith)
  crate.

* <input type="checkbox"/> Add a `wasmtime::Config::enable_foo_bar` method to
  the `wasmtime` crate.

* <input type="checkbox"/> Add a `--enable-foo-bar` command line flag to the
  `wasmtime` binary.

* <input type="checkbox"/> Enable the spec tests in
  [`build.rs`](https://github.com/bytecodealliance/wasmtime/blob/c7cd70fcec3eee66c9d7b5aa6fb4580d5a802218/build.rs#L41-L52)
  but [mark them as
  ignored](https://github.com/bytecodealliance/wasmtime/blob/c7cd70fcec3eee66c9d7b5aa6fb4580d5a802218/build.rs#L196)
  for now.

* <input type="checkbox"/> Stop ignoring individual spec tests and get them
  passing one by one.

* <input type="checkbox"/> Enable the proposal in [the fuzz
  targets](./contributing-fuzzing.html).

  * <input type="checkbox"/> Add examples from the spec tests to [the relevant
    corpora](https://github.com/bytecodealliance/wasmtime-libfuzzer-corpus).

    > The `wast2json` tool from [WABT] is useful for this.

  * <input type="checkbox"/> Write a custom fuzz target, oracle, and/or test
    case generator for fuzzing this proposal in particular.

    > For example, we wrote a [custom
    > generator](https://github.com/bytecodealliance/wasmtime/blob/c7cd70fcec3eee66c9d7b5aa6fb4580d5a802218/crates/fuzzing/src/generators/table_ops.rs),
    > [oracle](https://github.com/bytecodealliance/wasmtime/blob/c7cd70fcec3eee66c9d7b5aa6fb4580d5a802218/crates/fuzzing/src/oracles.rs#L417-L467),
    > and [fuzz
    > target](https://github.com/bytecodealliance/wasmtime/blob/c7cd70fcec3eee66c9d7b5aa6fb4580d5a802218/fuzz/fuzz_targets/table_ops.rs)
    > for exercising `table.{get,set}` instructions and their interaction with
    > GC while implementing the reference types proposal.

* <input type="checkbox"/> Expose the proposal's new functionality in the
  `wasmtime` crate's API.

  > For example, the bulk memory operations proposal introduced a `table.copy`
  > instruction, and we exposed its functionality as the `wasmtime::Table::copy`
  > method.

* <input type="checkbox"/> Expose the proposal's new functionality in the C API.

  > This may require extensions to the standard C API, and if so, should be
  > defined in
  > [`wasmtime.h`](https://github.com/bytecodealliance/wasmtime/blob/c7cd70fcec3eee66c9d7b5aa6fb4580d5a802218/crates/c-api/include/wasmtime.h)
  > and prefixed with `wasmtime_`.

* <input type="checkbox"/> Use the C API to expose the proposal's new
  functionality in the other language embedding APIs:

  * <input type="checkbox"/> [Python](https://github.com/bytecodealliance/wasmtime-py/)

  * <input type="checkbox"/> [Go](https://github.com/bytecodealliance/wasmtime-go/)

  * <input type="checkbox"/> [.NET](https://github.com/bytecodealliance/wasmtime-dotnet/)

* <input type="checkbox"/> Document support for the proposal in
  `wasmtime/docs/stability-wasm-proposals-support.md`.

## Enabling Support for a Proposal by Default

These are the standards that must be met to enable support for a proposal by
default in Wasmtime, and can be used as a review checklist.

* <input type="checkbox"/> The proposal must be in phase 4, or greater, of [the
  WebAssembly standardization process][phases].

* <input type="checkbox"/> All spec tests must be passing in Wasmtime.

* <input type="checkbox"/> No open questions, design concerns, or serious known
  bugs.

* <input type="checkbox"/> Has been fuzzed for at least a week minimum.

* <input type="checkbox"/> We are confident that the fuzzers are fully
  exercising the proposal's functionality.

  > For example, it would *not* have been enough to simply enable reference
  > types in the `compile` fuzz target to enable that proposal by
  > default. Compiling a module that uses reference types but not instantiating
  > it nor running any of its functions doesn't exercise any of the GC
  > implementation and does not run the inline fast paths for `table` operations
  > emitted by the JIT. Exercising these things was the motivation for writing
  > the custom fuzz target for `table.{get,set}` instructions.

* <input type="checkbox"/> The proposal's functionality is exposed in the
  `wasmtime` crate's API.

* <input type="checkbox"/> The proposal's functionality is exposed in the C API.

* <input type="checkbox"/> The proposal's functionality is exposed in at least
  one of the other languages' APIs.

[phases]: https://github.com/WebAssembly/meetings/blob/master/process/phases.md
[WABT]: https://github.com/WebAssembly/wabt/

## Adding component functionality to WASI
The [cap-std](https://github.com/bytecodealliance/cap-std) repository contains
crates which implement the capability-based version of the Rust standard library
and extensions to that functionality. Once the functionality has been added to
the relevant crates of that repository, they can be added into wasmtime by
including them in the preview2 directory of the [wasi crate](https://github.com/bytecodealliance/wasmtime/tree/main/crates/wasi).

Currently, WebAssembly modules which rely on preview2 ABI cannot be directly
executed by the wasmtime command. The following steps allow for testing such
changes.

1. Build wasmtime with the changes `cargo build --release`

2. Create a simple Webassembly module to test the new component functionality by
compiling your test code to the `wasm32-wasip1` build target.

3. Build the [wasi-preview1-component-adapter](https://github.com/bytecodealliance/wasmtime/tree/main/crates/wasi-preview1-component-adapter)
as a command adapter. `cargo build -p wasi-preview1-component-adapter --target
wasm32-wasip1 --release --features command --no-default-features`

4. Use [wasm-tools](https://github.com/bytecodealliance/wasm-tools) to convert
the test module to a component. `wasm-tools component new --adapt
wasi_snapshot_preview1=wasi_snapshot_preview1.command.wasm -o component.wasm
path/to/test/module`

5. Run the test component created in the previous step with the locally built
wasmtime. `wasmtime -W component-model=y -S preview2=y component.wasm`
