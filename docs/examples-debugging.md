# Debugging WebAssembly

Wasmtime currently provides the following support for debugging misbehaving
WebAssembly:

* We can [live debug and step through the guest Wasm and the host at the same
  time with `gdb` or `lldb`.](./examples-debugging-native-debugger.md)

* When a Wasm guest traps, we can [generate Wasm core
  dumps](./examples-debugging-core-dumps.md), that can be consumed by other
  tools for post-mortem analysis.
