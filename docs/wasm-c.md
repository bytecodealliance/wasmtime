# C/C++

All the parts needed to support wasm are included in upstream clang, lld, and
compiler-rt, as of the LLVM 8.0 release. However, to use it, you'll need
to build WebAssembly-targeted versions of the library parts, and it can
be tricky to get all the CMake invocations lined up properly.

To make things easier, we provide
[prebuilt packages](https://github.com/CraneStation/wasi-sdk/releases)
that provide builds of Clang and sysroot libraries.

WASI doesn't yet support `setjmp`/`longjmp` or C++ exceptions, as it is
waiting for [unwinding support in WebAssembly].

[unwinding support in WebAssembly]: https://github.com/WebAssembly/exception-handling/
