# Interrupting Wasm Execution

If you want to interrupt Wasm execution, for example to prevent an infinite loop
in the Wasm guest from indefinitely blocking the host, Wasmtime provides two
mechanisms you can choose between. Wasmtime also allows you to choose what
happens when Wasm execution is interrupted.

## What Happens When Execution is Interrupted

When a Wasm program's execution is interrupted, you can configure Wasmtime to do
either of the following:

* **Raise a trap:** This terminates the current Wasm program, and it is not
  resumable.

* **Async yield:** This pauses the current Wasm program, yields control back to
  the host, and lets the host decide whether to resume execution sometime in the
  future.

These options are both available regardless of which interruption mechanism you
employ.

## Interruption Mechanisms

### Deterministic Fuel

Fuel-based interruption is completely deterministic: the same program run with
the same amount of fuel will always be interrupted at the same location in the
program (unless it has enough fuel to complete its computation, or there is some
other form of non-determinism that causes the program to behave differently).

The downside is that fuel-based interruption imposes more overhead on execution,
slowing down Wasm programs, than epochs do.

```rust,ignore
{{#include ../examples/fuel.rs}}
```

See these API docs for more details:

* [`wasmtime::Config::consume_fuel`](https://docs.rs/wasmtime/latest/wasmtime/struct.Config.html#method.consume_fuel)
* [`wasmtime::Config::set_fuel`](https://docs.rs/wasmtime/latest/wasmtime/struct.Store.html#method.set_fuel)
* [`wasmtime::Config::fuel_async_yield_interval`](https://docs.rs/wasmtime/latest/wasmtime/struct.Store.html#method.fuel_async_yield_interval)

### Non-Deterministic Epochs

Epoch-based interruption imposes relatively low overhead on Wasm execution; it
has been measured at around a 10% slowdown. It is faster than fuel-based
interruption.

The downside is that it is non-deterministic. Running the same program with the
same inputs for one epoch might result in an interrupt at one location the first
time, a later location the second time, or even complete successfully another
time. This is because it is based on wall-time rather than an exact count of how
many Wasm instructions are executed.

```rust,ignore
{{#include ../examples/epochs.rs}}
```

See these API docs for more details:

* [`wasmtime::Config::epoch_interruption`](https://docs.rs/wasmtime/latest/wasmtime/struct.Config.html#method.epoch_interruption)
* [`wasmtime::Config::epoch_deadline_trap`](https://docs.rs/wasmtime/latest/wasmtime/struct.Store.html#method.epoch_deadline_trap)
* [`wasmtime::Config::epoch_deadline_callback`](https://docs.rs/wasmtime/latest/wasmtime/struct.Store.html#method.epoch_deadline_callback)
* [`wasmtime::Config::epoch_deadline_async_yield_and_update`](https://docs.rs/wasmtime/latest/wasmtime/struct.Store.html#method.epoch_deadline_async_yield_and_update)
