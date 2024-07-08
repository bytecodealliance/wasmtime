# Wasmtime's C API

For more information you can find the documentation for this library
[online](https://bytecodealliance.github.io/wasmtime/c-api/).

## Using in a C Project

To use Wasmtime from a C or C++ project, you can use Cargo to build the Wasmtime C bindings. From the root of the Wasmtime repository, run the following command:

```
cargo build --release -p wasmtime-c-api
```

This will create static and dynamic libraries called `libwasmtime` in the `target/release` directory.

## Using in a Rust Project

If you have a Rust crate that contains bindings to a C or C++ library that uses Wasmtime, you can link the Wasmtime C API using Cargo.

1. Add a dependency on the `wasmtime-c-api-impl` crate to your `Cargo.toml`. Note that package name differs from the library name.

```toml
[dependencies]
wasmtime-c-api = { version = "16.0.0", package = "wasmtime-c-api-impl" }
```

2. In your `build.rs` file, when compiling your C/C++ source code, add the C `wasmtime-c-api` headers to the include path:

```rust
fn main() {
    let mut cfg = cc::Build::new();

    // Add to the include path the wasmtime headers and the standard
    // Wasm C API headers.
    cfg
        .include(std::env::var("DEP_WASMTIME_C_API_INCLUDE").unwrap());
        .include(std::env::var("DEP_WASMTIME_C_API_WASM_INCLUDE").unwrap());

    // Compile your C code.
    cfg
        .file("src/your_c_code.c")
        .compile("your_library");
}
```
