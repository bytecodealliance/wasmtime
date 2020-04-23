# Profiling WebAssembly

One of WebAssembly's major goals is to be quite close to native code in terms of
performance, so typically when executing wasm you'll be quite interested in how
well your wasm module is performing! From time to time you might want to dive a
bit deeper into the performance of your wasm, and this is where profiling comes
into the picture.

Profiling support in Wasmtime is still under development, but if you're using either [perf](./examples-profiling-perf.md) or [Vtune](./examples-profiling-vtune.md) the examples in these sections are targeted at helping you get some information about the performance of your wasm modules.
