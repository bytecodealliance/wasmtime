# WASI Proposals Support

Wasmtime has initial support for some WASI proposals to assist in their
development as well as allowing users to more easily provide feedback by testing
them. This is somewhat of a work-in-progress because there aren't a lot of WASI
proposals at this time.

Currently supported WASI proposals are:

* [The base WASI specification](https://github.com/webassembly/wasi) - the
  `snapshot` directory is supported by the [`wasmtime-wasi`
  crate](https://docs.rs/wasmtime-wasi/0.21.0/wasmtime_wasi/)

* [The wasi-nn proposal](https://github.com/WebAssembly/wasi-nn) - the
  `ephemeral` directory with the wasi-nn APIs.

Wasmtime strives to support WASI proposals in-tree to assist with
maintainability and provide easier to support to users for easier feedback. In
doing so, however, we have a few guidelines around adding support for a WASI
proposal to Wasmtime:

* The proposal must be tested and exercised on CI. These tests don't need to be
  overly exhaustive but a "smoke test" should pass. Additionally CI should not
  be overly long compared to existing tests run, and it should also strive to be
  relataively reliable. Wasmtime contributors may disable CI for proposals at
  any time if needed, and maintainers for the proposal will be notified.

* The size of the code in the wasmtime repository should not be unduly large.
  This helps keep the size of the repo within reasonable limits and relatively
  easy for others not working on the proposal to still be productive.

* It should be possible to compile wasmtime without support for the proposal.
  It's ok to compile it in by default (but this may not always be the right
  choice), but Wasmtime should always be able to compile without the proposal.
  Additionally the proposal doesn't need to support all platforms in the get-go.

We may tweak these guidelines over time but this is our working set of
assumptions for now!
