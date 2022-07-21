# Examples of the `wasmtime` API

This directory contains a number of examples of using the `wasmtime` API from
different languages. Currently examples are all in Rust and C using the
`wasmtime` crate or the wasmtime embedding API.

Each example is available in both C and in Rust. Examples are accompanied with a
`*.wat` file which is the wasm input, or a Rust project in a `wasm` folder which
is the source code for the original wasm file.

Rust examples can be executed with `cargo run --example $name`. C examples can
be built with `mkdir build && cd build && cmake ..`. You can run
`cmake --build .` to build all examples or
`cmake --build . --target wasmtime-$name`, replacing the name as you wish. They
can also be [built manually](https://docs.wasmtime.dev/c-api/).

For more information see the examples themselves!
