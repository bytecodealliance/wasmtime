# Examples of the `wasmtime` API

This directory contains a number of examples of using the `wasmtime` API from
different languages. Currently examples are all in Rust and C using the
`wasmtime` crate or the wasmtime embedding API.

Each example is available in both C and in Rust. Examples are accompanied with a
`*.wat` file which is the wasm input, or a Rust project in a `wasm` folder which
is the source code for the original wasm file.

Rust examples can be executed with `cargo run --example $name`, and C examples
need to be compiled using your system compiler and appropriate header files.

For more information see the examples themselves!
