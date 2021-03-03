# C/C++

All the parts needed to support wasm are included in upstream clang, lld, and
compiler-rt, as of the LLVM 8.0 release. However, to use it, you'll need
to build WebAssembly-targeted versions of the library parts, and it can
be tricky to get all the CMake invocations lined up properly.

To make things easier, we provide
[prebuilt packages](https://github.com/WebAssembly/wasi-sdk/releases)
that provide builds of Clang and sysroot libraries.

WASI doesn't yet support `setjmp`/`longjmp` or C++ exceptions, as it is
waiting for [unwinding support in WebAssembly].

By default, the C/C++ toolchain orders linear memory to put the globals first,
the stack second, and start the heap after that. This reduces code size,
because references to globals can use small offsets. However, it also means
that stack overflow will often lead to corrupted globals. The
`-Wl,--stack-first` flag to clang instructs it to put the stack first, followed
by the globals and the heap, which may produce slightly larger code, but will
more reliably trap on stack overflow.

[unwinding support in WebAssembly]: https://github.com/WebAssembly/exception-handling/
