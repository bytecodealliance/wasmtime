# Linking modules

You can also [browse this source code online][code] and clone the wasmtime
repository to run the example locally.

[code]: https://github.com/bytecodealliance/wasmtime/blob/master/examples/linking.rs

This example shows off how to compile and instantiate modules which link
together.

## `linking1.wat`

```wat
{{#include ../examples/linking1.wat}}
```

## `linking2.wat`

```wat
{{#include ../examples/linking2.wat}}
```

## `linking.rs`

```rust,ignore
{{#include ../examples/linking.rs}}
```
