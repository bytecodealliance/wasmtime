# Tuning Wasmtime for Fast Compilation

Wasmtime must compile a Wasm program before executing it. This means that, by
default, Wasm compilation is on your critical path. In most scenarios, you can
completely remove Wasm compilation from the critical path by [pre-compiling Wasm
programs](./examples-pre-compiling-wasm.md). That option is not always
available, however, and this page documents how to tune Wasmtime for fast
compilation in these alternative scenarios.

## Enable the Compilation Cache

Wasmtime can be configured to use a cache, so that if you attempt to compile a
Wasm program that has already been compiled previously, it just grabs the cached
result rather than performing compilation all over again.

See these API docs for more details:

* [`wasmtime::Config::cache`](https://docs.rs/wasmtime/latest/wasmtime/struct.Config.html#method.cache)
* [`wasmtime::CacheStore`](https://docs.rs/wasmtime/latest/wasmtime/trait.CacheStore.html)

## Enable Winch

Winch is Wasmtime's "baseline" compiler: for each Wasm opcode, it emits a canned
sequence of machine instructions to implement that opcode. This makes
compilation fast: it performs only a single, quick pass over the Wasm
code. However, it does not perform optimizations, so the machine code it emits
will run Wasm programs slower than Cranelift, Wasmtime's optimizing compiler.

See the API docs for
[`wasmtime::Strategy`](https://docs.rs/wasmtime/latest/wasmtime/enum.Strategy.html)
and
[`wasmtime::Config::strategy`](https://docs.rs/wasmtime/latest/wasmtime/struct.Config.html#method.strategy)
for more details.

## Enable Parallel Compilation

Wasmtime can compile Wasm programs in parallel, speeding up the compilation
process more or less depending on how many cores your machine has and the exact
shape of the Wasm program. Wasmtime will generally enable parallel compilation
by default, but it does depend on the host platform and cargo features enabled
when building Wasmtime itself. You can double check that parallel compilation is
enabled via the setting the
[`wasmtime::Config::parallel_compilation`](https://docs.rs/wasmtime/latest/wasmtime/struct.Config.html#method.parallel_compilation)
configuration option.

## Putting It All Together

```rust,ignore
{{#include ../examples/fast_compilation.rs}}
```

## See Also

* [Pre-Compiling Wasm Programs](./examples-pre-compiling-wasm.md)
* [Tuning Wasmtime for Fast Wasm Instantiation](./examples-fast-instantiation.md)
* [Tuning Wasmtime for Fast Wasm Execution](./examples-fast-execution.md)
