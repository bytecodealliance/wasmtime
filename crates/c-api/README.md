# Wasmtime's C API

For more information you can find the documentation for this library
[online](https://bytecodealliance.github.io/wasmtime/c-api/).

## Using in a C Project

### Using a Pre-Built Static or Dynamic Library

Each release on Wasmtime's [GitHub Releases
page](https://github.com/bytecodealliance/wasmtime/releases) has pre-built
binaries for both static and dynamic libraries for a variety of architectures
and operating systems attached, as well as header files you can include.

### Building Wasmtime's C API from Source

To use Wasmtime from a C or C++ project, you must have
[CMake](https://cmake.org/) and [a Rust
toolchain](https://www.rust-lang.org/tools/install) installed.

From the root of the Wasmtime repository, run the following commands:

```
$ cmake -S crates/c-api -B target/c-api
$ cmake --build target/c-api
```

These commands will produce the following files:

* `target/<triple>/release/libwasmtime.{a,lib}`: Static Wasmtime library. Exact
  extension depends on your operating system. `<triple>` is your platform's
  target triple, such as `x86_64-unknown-linux-gnu`.

* `target/<triple>/release/libwasmtime.{so,dylib,dll}`: Dynamic Wasmtime
  library. Exact extension depends on your operating system.  `<triple>` is your
  platform's target triple, such as `x86_64-unknown-linux-gnu`.

* `target/c-api/include/wasmtime/conf.h`: A header file that tells the main
  Wasmtime header which optional features were compiled into these libraries.

* `crates/c-api/html/index.html`: Doxygen documentation for the Wasmtime C API.

Other header files you will want:

* `crates/c-api/include/wasmtime.h`
* `crates/c-api/include/wasmtime/*.h`

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
