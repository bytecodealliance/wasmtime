# Serializing and Deserializing Modules

You can also [browse this source code online][code] and clone the wasmtime
repository to run the example locally.

[code]: https://github.com/bytecodealliance/wasmtime/blob/main/examples/serialize.cc

This example shows how to serialize a compiled module to disk and later deserialize it to skip compilation.

## `serialize.cc`

```cpp
{{#include ../examples/serialize.cc}}
```
