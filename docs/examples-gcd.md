# Calculating the GCD

You can also browse this source code online and clone the wasmtime
repository to run the example locally:

* [Rust](https://github.com/bytecodealliance/wasmtime/blob/main/examples/gcd.rs)
* [C](https://github.com/bytecodealliance/wasmtime/blob/main/examples/gcd.c)
* [C++](https://github.com/bytecodealliance/wasmtime/blob/main/examples/gcd.cc)

This example shows off how run a wasm program which calculates the GCD of two
numbers.

## Wasm Source

```wat
{{#include ../examples/gcd.wat}}
```

## Host Source

<!-- langtabs-start -->

```rust
{{#include ../examples/gcd.rs}}
```

```c
{{#include ../examples/gcd.c}}
```

```cpp
{{#include ../examples/gcd.cc}}
```

<!-- langtabs-end -->
