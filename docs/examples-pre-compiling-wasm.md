# Pre-Compiling and Cross-Compiling WebAssembly Programs

Wasmtime can compile a WebAssembly program to native code on one machine, and
then run it on a different machine. This has a number of benefits:

* **Faster start up:** Compilation is removed from the critical path. When a new
  HTTP request comes into your function-as-a-service platform, for example, you
  do not have to wait for the associated Wasm program to compile before it can
  start handling the request. Similarly, when a new update for your embedded
  device's Wasm application logic comes in, you do not need to compile the
  update on the under-powered device before it can begin running new updated
  logic.

* **Less Memory Usage:** Pre-compiled Wasm programs can be lazily `mmap`ed from
  disk, only paging their code into memory as those code paths are executed. If
  none of the code on a page is ever executed, the OS will never make the page
  resident. This means that running pre-compiled Wasm programs lowers overall
  memory usage in the system.

* **Smaller Code Size for Embedded:** Wasmtime can be built such that it can
  *only* run Wasm programs that were pre-compiled elsewhere. These builds will
  not include the executable code for Wasm compilation. This is done by
  disabling the `cranelift` and `winch` cargo features at build time. These
  builds are useful for embedded devices, where programs must be small and fit
  within the device's constrained environment.

* **Smaller Attack Surfaces:** Similarly, building Wasmtime without a compiler,
  and with only support for running pre-compiled Wasm programs, can be useful
  for security-minded embeddings to reduce the potential attack surface exposed
  to untrusted and potentially hostile Wasm guests. Compilation, triggered by
  the control plane, can happen inside a Wasmtime build that can compile but not
  run Wasm programs. Execution, in the data plane, can happen inside a Wasmtime
  build that can run but not compile new Wasm programs. Exposing a minimal
  attack surface to untrusted code is good security practice.

Note that these benefits are applicable regardless which Wasm execution strategy
you've configured: Cranelift, Winch, or Pulley.

## Pre-Compile the Wasm on One Machine

This must be done with a Wasmtime build that has a Wasm execution strategy
enabled, e.g. was built with the `cranelift` or `winch` cargo features. It does
not require the ability to run Wasm programs, so the `runtime` cargo feature can
be disabled at build time.

```rust,ignore
{{#include ../examples/pre_compile.rs}}
```

## Run the Pre-Compiled Wasm on Another Machine

This must be done with a Wasmtime build that can run pre-compiled Wasm programs,
that is a Wasmtime built with the `runtime` cargo feature. It does not need to
compile new Wasm programs, so the `cranelift` and `winch` cargo features can be
disabled.

```rust,ignore
{{#include ../examples/run_pre_compiled.rs}}
```

## See Also

* [Tuning Wasmtime for Fast Wasm Instantiation](./examples-fast-instantiation.md)
* [Tuning Wasmtime for Fast Wasm Execution](./examples-fast-execution.md)
* [Building a Minimal Wasmtime Embedding](./examples-minimal.md)
* [`wasmtime::Engine::precompile_module`](https://docs.rs/wasmtime/latest/wasmtime/struct.Engine.html#method.precompile_module)
  and
  [`wasmtime::Engine::precompile_component`](https://docs.rs/wasmtime/latest/wasmtime/struct.Engine.html#method.precompile_component)
* [`wasmtime::Module::deserialize`](https://docs.rs/wasmtime/latest/wasmtime/struct.Module.html#method.deserialize),
  [`wasmtime::Module::deserialize_file`](https://docs.rs/wasmtime/latest/wasmtime/struct.Module.html#method.deserialize_file),
  [`wasmtime::component::Component::deserialize`](https://docs.rs/wasmtime/latest/wasmtime/component/struct.Component.html#method.deserialize),
  and
  [`wasmtime::component::Component::deserialize_file`](https://docs.rs/wasmtime/latest/wasmtime/component/struct.Component.html#method.deserialize_file)
