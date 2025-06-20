# Tuning Wasmtime for Fast Instantiation

Before a WebAssembly module can begin execution, it must first be compiled and
then instantiated. Compilation can happen [ahead of
time](./examples-pre-compiling-wasm.md), which removes compilation from the
critical path. That leaves just instantiation on the critical path. This page
documents methods for tuning Wasmtime for fast instantiation.

## Enable the Pooling Allocator

By enabling the pooling allocator, you are configuring Wasmtime to up-front and
ahead-of-time allocate a large pool containing all the resources necessary to
run the configured maximum number of concurrent instances. Creating a new
instance doesn't require allocating new Wasm memories and tables on demand, it
just takes pre-allocated memories and tables from the pool, which is generally
much faster. Deallocating an instance returns its memories and tables to the
pool.

See
[`wasmtime::PoolingAllocationConfig`](https://docs.rs/wasmtime/latest/wasmtime/struct.PoolingAllocationConfig.html),
[`wasmtime::InstanceAllocationStrategy`](https://docs.rs/wasmtime/latest/wasmtime/enum.InstanceAllocationStrategy.html),
and
[`wasmtime::Config::allocation_strategy`](https://docs.rs/wasmtime/latest/wasmtime/struct.Config.html#method.allocation_strategy)
for more details.

## Enable Copy-on-Write Heap Images

Initializing a WebAssembly linear memory via a copy-on-write mapping can
drastically improve instantiation costs because copying memory is deferred from
instantiation time to when the data is first mutated. When the Wasm module only
reads the initial data, and never overwrites it, then the copying is completely
avoided.

See the API docs for
[`wasmtime::Config::memory_init_cow`](https://docs.rs/wasmtime/latest/wasmtime/struct.Config.html#method.memory_init_cow)
for more details.

## Use `InstancePre`

To instantiate a WebAssembly module or component, Wasmtime must look up each of
the module's imports and check that they are of the expected types. If the
imports are always the same, this work can be done ahead of time, before
instantiation. A `wasmtime::InstancePre` represents an instance *just before* it
is instantiated, after all type-checking and imports have been resolved. The
only thing left to do for this instance is to actually allocate its memories,
tables, and internal runtime context, initialize its state, and run its `start`
function, if any.

See the API docs for
[`wasmtime::InstancePre`](https://docs.rs/wasmtime/latest/wasmtime/struct.InstancePre.html),
[`wasmtime::Linker::instantiate_pre`](https://docs.rs/wasmtime/latest/wasmtime/struct.Linker.html#method.instantiate_pre),
[`wasmtime::component::InstancePre`](https://docs.rs/wasmtime/latest/wasmtime/component/struct.InstancePre.html),
and
[`wasmtime::component::Linker::instantiate_pre`](https://docs.rs/wasmtime/latest/wasmtime/component/struct.Linker.html#method.instantiate_pre)
for more details.

## Putting It All Together

```rust,ignore
{{#include ../examples/fast_instantiation.rs}}
```

## See Also

* [Pre-Compiling Wasm Programs](./examples-pre-compiling-wasm.md)
* [Tuning Wasmtime for Fast Wasm Compilation](./examples-fast-compilation.md)
* [Tuning Wasmtime for Fast Wasm Execution](./examples-fast-execution.md)
* [Wizer, the WebAssembly Pre-Initializer](https://github.com/bytecodealliance/wizer)
