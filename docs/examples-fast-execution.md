# Tuning Wasmtime for Fast Wasm Execution

To tune Wasmtime for faster Wasm execution, consider the following tips.

## Enable Cranelift

[Cranelift](https://cranelift.dev/) is an optimizing compiler. Compared to
alternative strategies like [the Winch "baseline"
compiler](./examples-fast-compilation.md), it translates Wasm into faster
machine code, but compilation is slower. Cranelift is similar to the optimizing
tier of browsers' just-in-time Wasm engines, such as SpiderMonkey's Ion tier or
V8's TurboFan tier.

See the API docs for
[`wasmtime::Strategy`](https://docs.rs/wasmtime/latest/wasmtime/enum.Strategy.html)
and
[`wasmtime::Config::strategy`](https://docs.rs/wasmtime/latest/wasmtime/struct.Config.html#method.strategy)
for more details.

## Configure Wasmtime to Elide Explicit Bounds Checks

Wasm programs are sandboxed and may only access their linear memories. Attempts
to access beyond the bounds of a linear memory results in a trap, and this
prevents the Wasm guest from stealing data from host memory, or from another
concurrently running Wasm instance. Explicitly bounds-checking every linear
memory operation performed by a Wasm guest is expensive: it has been measured to
create between a 1.2x to 1.8x slow down, depending on a number of
factors. Luckily, Wasmtime can usually omit explicit bounds checks by relying on
virtual memory guard pages. This requires enabling signals-based traps (on by
default for non-bare-metal builds), running Wasm on a 64-bit host architecture,
and ensuring that memory reservations and guard pages are appropriately sized
(again, configured by default for 64-bit architectures).

To elide any explicit bounds checks, Wasm linear memories must have at least a
4GiB (`1 << 32` bytes) reservation. If a memory instruction has an additional
static offset immediate, then the bounds check can only be elided when there is
a memory guard of at least that offset's size. Using a 4GiB guard region allows
Wasmtime to elide explicit bounds checks regardless of the static memory offset
immediate. While small static offset immediate values are common, very large
values are exceedingly rare, so you can get almost all of the benefits while
consuming less virtual memory address space by using, for example, 32MiB guards.

See the API docs for
[`wasmtime::Config::signals_based_traps`](https://docs.rs/wasmtime/latest/wasmtime/struct.Config.html#method.signals_based_traps),
[`wasmtime::Config::memory_reservation`](https://docs.rs/wasmtime/latest/wasmtime/struct.Config.html#method.memory_reservation),
and
[`wasmtime::Config::memory_reservation`](https://docs.rs/wasmtime/latest/wasmtime/struct.Config.html#method.memory_guard_size)
for more details.

## Force-Enable ISA Extensions

This section can be ignored if you are compiling and running Wasm programs on
the same machine. In this scenario, Wasmtime will automatically detect which ISA
extensions (such as AVX on x86\_64) are available, and you do not need to
configure anything yourself.

However, if you are compiling a Wasm program on one machine and then running
that pre-compiled Wasm program on another machine, then during compilation
Wasmtime cannot automatically detect which ISA extensions will be available on
the machine on which you actually execute the pre-compiled Wasm
program. Configuring which ISA extensions are available on the target
architecture that will run the pre-compiled Wasm programs can have a large
impact for certain Wasm programs, particularly those using SIMD instructions.

See the API docs for
[`wasmtime::Config::cranelift_flag_enable`](https://docs.rs/wasmtime/latest/wasmtime/struct.Config.html#method.cranelift_flag_enable)
for more details.

## Putting It All Together

```rust,ignore
{{#include ../examples/fast_execution.rs}}
```

## See Also

* [Tuning Wasmtime for Fast Wasm Compilation](./examples-fast-compilation.md)
* [Tuning Wasmtime for Fast Wasm Instantiation](./examples-fast-instantiation.md)
* [Pre-Compiling Wasm Programs](./examples-pre-compiling-wasm.md)
