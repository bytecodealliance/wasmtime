# Examples of the `wasmtime` API

This directory contains a number of examples of using the `wasmtime` API from
different languages.

Most examples are available in Rust, C, and C++, using the `wasmtime` crate or the 
[C/C++ embedding API](https://docs.wasmtime.dev/c-api/). Examples are accompanied by a
`*.wat` file which is the wasm input, or a Rust project in a `wasm` folder which
is the source code for the original wasm file.

Rust examples can be executed with `cargo run --example $name`. C and C++ examples
can be built with `cmake -B build -S examples` followed by `cmake --build build`
to build all examples, or `cmake --build build --target wasmtime-$name` to build
a single one, where for C `$name` is the basename of the example, and for C++
it is `[basename]-cpp`. The resulting executable is placed in `build/`.
They can also be [built manually](https://docs.wasmtime.dev/c-api/).

For more information see the examples themselves!
