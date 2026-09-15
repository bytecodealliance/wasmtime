# Implementing WebAssembly Proposals

## Adding New Support for a Wasm Proposal

The following checkboxes enumerate the steps required to add support for a new
WebAssembly proposal to Wasmtime. They can be completed over the course of
multiple pull requests.

* [ ] Implement support for the proposal in the [`wasm-tools` repository].
  [(example)](https://github.com/bytecodealliance/wasm-tools/pull/1853)
  * [ ] [`wast`] - text parsing
  * [ ] [`wasmparser`] - binary decoding and validation
  * [ ] [`wasmprinter`] - binary-to-text
  * [ ] [`wasm-encoder`] - binary encoding
  * [ ] [`wasm-smith`] - fuzz test case generation
* [ ] Update Wasmtime to use these `wasm-tools` crates, but leave the new
  proposal unimplemented for now (implementation comes in subsequent PRs).
  [(example)](https://github.com/bytecodealliance/wasmtime/pull/9399)
* [ ] Add `Config::wasm_your_proposal` to the `wasmtime` crate.
  * [ ] If the proposal adds a `WasmFeatures` flag, include it in
    `features_known_to_wasmtime` in
    [`crates/wasmtime/src/config.rs`](https://github.com/bytecodealliance/wasmtime/blob/0675528cca48cc8dd90cefe91eb7f1a279a7415b/crates/wasmtime/src/config.rs).
* [ ] Implement the proposal in `wasmtime`, gated behind this flag.
* [ ] Add `your-proposal` to `WasmOptions` in the `wasmtime-cli-flags`
  crate, apply it to `wasmtime::Config`, and decide whether it should be
  included in `-W all-proposals`.
* [ ] Configure the WAST test harness for the proposal in
  `crates/test-util/src/wast.rs` and
  `crates/test-util/src/wasmtime_wast.rs`.
* [ ] Write custom tests in `tests/misc_testsuite/*.wast` for this proposal.
* [ ] Enable the proposal in [the fuzz targets](./contributing-fuzzing.md).
  * [ ] Write a custom fuzz target, oracle, and/or test
    case generator for fuzzing this proposal in particular.

    > For example, we wrote a [custom
    > generator](https://github.com/bytecodealliance/wasmtime/blob/c7cd70fcec3eee66c9d7b5aa6fb4580d5a802218/crates/fuzzing/src/generators/table_ops.rs),
    > [oracle](https://github.com/bytecodealliance/wasmtime/blob/c7cd70fcec3eee66c9d7b5aa6fb4580d5a802218/crates/fuzzing/src/oracles.rs#L417-L467),
    > and [fuzz
    > target](https://github.com/bytecodealliance/wasmtime/blob/c7cd70fcec3eee66c9d7b5aa6fb4580d5a802218/fuzz/fuzz_targets/table_ops.rs)
    > for exercising `table.{get,set}` instructions and their interaction with
    > GC while implementing the reference types proposal.
* [ ] Expose the proposal's new functionality in the `wasmtime` crate's API.

  > For example, the bulk memory operations proposal introduced a `table.copy`
  > instruction, and we exposed its functionality as the `wasmtime::Table::copy`
  > method.
* [ ] Expose the proposal's new functionality in the C API.

  > This may require extensions to the standard C API. Configuration extensions
  > should be declared in
  > [`crates/c-api/include/wasmtime/config.h`](https://github.com/bytecodealliance/wasmtime/blob/0675528cca48cc8dd90cefe91eb7f1a279a7415b/crates/c-api/include/wasmtime/config.h)
  > and prefixed with `wasmtime_`. Implement them in
  > [`crates/c-api/src/config.rs`](https://github.com/bytecodealliance/wasmtime/blob/0675528cca48cc8dd90cefe91eb7f1a279a7415b/crates/c-api/src/config.rs),
  > and add corresponding C++ wrappers to
  > [`crates/c-api/include/wasmtime/config.hh`](https://github.com/bytecodealliance/wasmtime/blob/0675528cca48cc8dd90cefe91eb7f1a279a7415b/crates/c-api/include/wasmtime/config.hh)
  > when needed.
* [ ] Update `docs/stability-wasm-proposals.md` with the proposal's
  implementation status.


[`wasm-tools` repository]: https://github.com/bytecodealliance/wasm-tools
[`wasmparser`]: https://github.com/bytecodealliance/wasm-tools/tree/main/crates/wasmparser
[`wast`]: https://github.com/bytecodealliance/wasm-tools/tree/main/crates/wast
[`wasmprinter`]: https://github.com/bytecodealliance/wasm-tools/tree/main/crates/wasmprinter
[`wasm-encoder`]: https://github.com/bytecodealliance/wasm-tools/tree/main/crates/wasm-encoder
[`wasm-smith`]: https://github.com/bytecodealliance/wasm-tools/tree/main/crates/wasm-smith

For information about the status of implementation of various proposals in
Wasmtime see the [associated documentation](./stability-wasm-proposals.md).

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
