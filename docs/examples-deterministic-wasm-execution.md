# Deterministic Wasm Execution

The WebAssembly language is *mostly* deterministic, but there are a few places
where non-determinism slips in. This page documents how to use Wasmtime to
execute Wasm programs fully deterministically, even when the Wasm language spec
allows for non-determinism.

## Make Sure All Imports are Deterministic

Do not give Wasm programs access to non-deterministic host functions.

When using WASI, use
[`wasi-virt`](https://github.com/bytecodealliance/WASI-Virt) to virtualize
non-deterministic APIs like clocks and file systems.

## Enable IEEE-754 `NaN` canonicalization

Some Wasm opcodes can result in `NaN` (not-a-number) values. The IEEE-754 spec
defines a whole range of `NaN` values and the Wasm spec does not require that
Wasm always generates any particular `NaN` value, it could be any one of
them. This non-determinism can then be observed by the Wasm program by storing
the `NaN` value to memory or bitcasting it to an integer. Therefore, Wasmtime
can be configured to canonicalize all `NaN`s into a particular, canonical `NaN`
value. The downside is that this adds overhead to Wasm's floating-point
instructions.

See
[wasmtime::Config::cranelift_nan_canonicalization](https://docs.rs/wasmtime/latest/wasmtime/struct.Config.html#method.cranelift_nan_canonicalization)
for more details.

## Make the Relaxed SIMD Proposal Deterministic

The relaxed SIMD proposal gives Wasm programs access to SIMD operations that
cannot be made to execute both identically and performantly across different
architectures. The proposal gave up determinism across different achitectures in
order to maintain portable performance.

At the cost of worse runtime performance, Wasmtime can deterministically execute
this proposal's instructions. See
[wasmtime::Config::relaxed_simd_deterministic](https://docs.rs/wasmtime/latest/wasmtime/struct.Config.html#method.relaxed_simd_deterministic)
for more details.

Alternatively, you can simply disable the proposal completely. See
[`wasmtime::Config::wasm_relaxed_simd`](https://docs.rs/wasmtime/latest/wasmtime/struct.Config.html#method.wasm_relaxed_simd)
for more details.

## Handling Non-Deterministic Memory and Table Growth

All WebAssembly memories and tables have an associated minimum, or initial, size
and an optional maximum size. When the maximum size is not present, that means
"unlimited". If a memory or table is already at its maximum size, then attempts
to grow them will always fail. If they are below their maximum size, however,
then the `memory.grow` and `table.grow` instructions are allowed to
non-deterministicaly succeed or fail (for example, when the host system does not
have enough memory available to satisfy that growth).

You can make this deterministic in a variety of ways:

* Disallow Wasm programs that use memories and tables via a
  [limiter](https://docs.rs/wasmtime/latest/wasmtime/struct.Store.html#method.limiter)
  that rejects non-zero-sized memories and tables.

* Use a [custom memory
  creator](https://docs.rs/wasmtime/latest/wasmtime/struct.Config.html#method.with_host_memory)
  that allocates the maximum size up front so that growth will either always
  succeed or fail before the program has begun execution.

* Use [the `wasmparser` crate](https://crates.io/crates/wasmparser) to write a
  little validator program that rejects Wasm modules that use
  `{memory,table}.grow` instructions or alternatively rejects memories and
  tables that do not have a maximum size equal to their minimum size (which,
  again, means that their allocation must happen completely up front, and if
  allocation fails, it will have failed before the Wasm program began
  executing).

## Use Deterministic Interruption, If Any

If you are making Wasm execution interruptible, use [deterministic fuel-based
interruption](./examples-interrupting-wasm.md#deterministic-fuel) rather than
non-deterministic epoch-based interruption.
