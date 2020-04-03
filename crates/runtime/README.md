This is the `wasmtime-runtime` crate, which contains wasm runtime library
support, supporting the wasm ABI used by [`wasmtime-environ`],
[`wasmtime-jit`], and [`wasmtime-obj`].

This crate does not make a host vs. target distinction; it is meant to be
compiled for the target.

Most users will want to use the main [`wasmtime`] crate instead of using this
crate directly.

[`wasmtime-environ`]: https://crates.io/crates/wasmtime-environ
[`wasmtime-jit`]: https://crates.io/crates/wasmtime-jit
[`wasmtime-obj`]: https://crates.io/crates/wasmtime-obj
[`wasmtime`]: https://crates.io/crates/wasmtime
