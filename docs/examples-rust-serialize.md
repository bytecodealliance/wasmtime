# Serializing and Deserializing Modules

You can also [browse this source code online][code] and clone the wasmtime
repository to run the example locally.

[code]: https://github.com/bytecodealliance/wasmtime/blob/main/examples/serialize.rs

This example shows how to compile a module once and serialize its compiled representation to disk and later deserialize it to skip compilation. See also the pre-compilation example for ahead-of-time compilation.

## `serialize.rs`

```rust,ignore
{{#include ../examples/serialize.rs}}
```
