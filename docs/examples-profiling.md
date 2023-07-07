# Profiling WebAssembly

One of WebAssembly's major goals is to be quite close to native code in terms of
performance, so typically when executing Wasm you'll be quite interested in how
well your Wasm module is performing! From time to time you might want to dive a
bit deeper into the performance of your Wasm, and this is where profiling comes
into the picture.

For best results, ideally you'd use hardware performance counters for your
timing measurements. However, that requires special support from your CPU and
operating system. Because Wasmtime is a JIT, that also requires hooks from
Wasmtime to your platform's native profiling tools.

As a result, Wasmtime support for native profiling is limited to certain
platforms. See the following sections of this book if you're using these
platforms:

- On Linux, we support [perf](./examples-profiling-perf.md).

- For Intel's x86 CPUs on Linux or Windows, we support
  [VTune](./examples-profiling-vtune.md).

- For everything else, see the cross-platform profiler below.

The native profilers can measure time spent in WebAssembly guest code as well as
time spent in the Wasmtime host and potentially even time spent in the kernel.
This provides a comprehensive view of performance.

If the native profiling tools don't work for you, Wasmtime also provides a
[cross-platform profiler](./examples-profiling-guest.md). This profiler can only
measure time spent in WebAssembly guest code, and its timing measurements are
not as precise as the native profilers. However, it works on every platform that
Wasmtime supports.
