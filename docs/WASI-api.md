# WASI API

Development of WASI has moved to the WASI CG Subgroup; see the
[WASI repository] and the [interfaces page on wasi.dev].

For Wasmtime embedders using the C API with the component model:

* Add core WASI imports to your component linker with
  [`wasmtime_component_linker_add_wasip2`].
* If your component imports `wasi:http`, also add
  [`wasmtime_component_linker_add_wasi_http`].
* Configure store-local WASI state with [`wasmtime_context_set_wasi`], and
  initialize WASI HTTP with [`wasmtime_context_set_wasi_http`] (after WASI).

See these headers for current signatures and feature-gating details:

* [`wasmtime/component/linker.h`]
* [`wasmtime/store.h`]

For legacy preview1 documentation, see the [preview1 docs].

[WASI repository]: https://github.com/WebAssembly/wasi
[interfaces page on wasi.dev]: https://wasi.dev/interfaces
[`wasmtime_component_linker_add_wasip2`]: https://github.com/bytecodealliance/wasmtime/blob/main/crates/c-api/include/wasmtime/component/linker.h
[`wasmtime_component_linker_add_wasi_http`]: https://github.com/bytecodealliance/wasmtime/blob/main/crates/c-api/include/wasmtime/component/linker.h
[`wasmtime_context_set_wasi`]: https://github.com/bytecodealliance/wasmtime/blob/main/crates/c-api/include/wasmtime/store.h
[`wasmtime_context_set_wasi_http`]: https://github.com/bytecodealliance/wasmtime/blob/main/crates/c-api/include/wasmtime/store.h
[`wasmtime/component/linker.h`]: https://github.com/bytecodealliance/wasmtime/blob/main/crates/c-api/include/wasmtime/component/linker.h
[`wasmtime/store.h`]: https://github.com/bytecodealliance/wasmtime/blob/main/crates/c-api/include/wasmtime/store.h
[preview1 docs]: https://github.com/WebAssembly/WASI/blob/main/legacy/preview1/docs.md
