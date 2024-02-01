# Using WebAssembly from Elixir

Wasmtime [is available on Hex](https://hex.pm/packages/wasmex) and can
be used programmatically to interact with Wasm modules. This guide will go over
installing the wasmex package and running a simple Wasm module from Elixir.

## Getting started and simple example

First, copy this example WebAssembly text module into the current directory. It exports
a function for calculating the greatest common denominator of two numbers.

```wat
{{#include ../examples/gcd.wat}}
```

The library has a Rust-based native extension, but thanks to `rustler_precompiled`, you
should not have to compile anything. It'll just work!

This WAT file can be executed in `iex`:

```elixir
Mix.install([:wasmex])
bytes = File.read!("gcd.wat")
{:ok, pid} = Wasmex.start_link(%{bytes: bytes}) # starts a GenServer running a WASM instance
Wasmex.call_function(pid, "gcd", [27, 6])
```

The last command should output:

```elixir
iex(5)> Wasmex.call_function(pid, "gcd", [27, 6])
{:ok, [3]}
```

If this is the output you see, congrats! You've successfully ran your first
WebAssembly code in Elixir!

## More examples and contributing

To learn more, check out an [another example](https://github.com/tessi/wasmex#example)
and the [API documentation](https://hexdocs.pm/wasmex/Wasmex.html).
If you have any questions, do not hesitate to open an issue on the
[GitHub repository](https://github.com/tessi/wasmex).
