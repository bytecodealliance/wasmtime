# Platform Support

This page is intended to give a high-level overview of Wasmtime's platform
support along with some aspirations of Wasmtime. For more details see the
documentation on [tiers of stability](./stability-tiers.md) which has specific
information about what's supported in Wasmtime on a per-matrix-combination
basis.

Wasmtime strives to support hardware that anyone wants to run WebAssembly on.
Wasmtime is intended to work out-of-the-box on most platforms by having
platform-specific defaults for the runtime. For example the native Cranelift
backend is enabled by default if supported, but otherwise the Pulley
interpreter backend is used if it's not supported.

## Compiler Support

Cranelift supports x86\_64, aarch64, s390x, and riscv64. No 32-bit platform is
currently supported. Building a new backend for Cranelift is a relatively large
undertaking which maintainers are willing to help with but it's recommended to
reach out to Cranelift maintainers first to discuss this.

Winch supports x86\_64. The aarch64 backend is in development. Winch is built on
Cranelift's support for emitting instructions so Winch's possible backend list
is currently limited to what Cranelift supports.

Usage of the Cranelift or Winch requires a host operating system which supports
creating executable memory pages on-the-fly. Support for statically linking in a
single precompiled module is not supported at this time.

Both Cranelift and Winch can be used either in AOT or JIT mode. In AOT mode one
process precompiles a module/component and then loads it into another process.
In JIT mode this is all done within the same process.

Neither Cranelift nor Winch support tiering at this time in the sense of having
a WebAssembly module start from a Winch compilation and automatically switch to
a Cranelift compilation. Modules are either entirely compiled with Winch or
Cranelift.

## Interpreter support

The `wasmtime` crate provides an implementation of a [WebAssembly interpreter
named "Pulley"](./examples-pulley.md) which is a portable implementation of
executing WebAssembly code. Pulley uses a custom bytecode which is created from
input WebAssembly similarly to how native architectures are supported. Pulley's
bytecode is created via a Cranelift backend for Pulley, so compile times for
the interpreter are expected to be similar to natively compiled code.

The main advantage of Pulley is that the bytecode can be executed on any
platform with the same pointer-width and endianness. For example to execute
Pulley on a 32-bit ARM platform you'd use the target `pulley32`. Similarly if
you wanted to run Pulley on x86\_64 you'd use the target `pulley64` for
Wasmtime.

Pulley's platform requirements are no greater than that of Wasmtime itself,
meaning that the goal is that if you can compile Wasmtime for a Rust target then
Pulley can run on that target.

Finally, note that while Pulley is optimized to be an efficient interpreter it
will never be as fast as native Cranelift backends. A performance penalty should
be expected when using Pulley.

## OS Support

Wasmtime with Pulley should work out-of-the-box on any Rust target, but for
optimal runtime performance of WebAssembly OS integration is required. In the
same way that Pulley is slower than a native Cranelift backend Wasmtime will be
slower on Rust targets it has no OS support for. Wasmtime will for example use
virtual memory when possible to implement WebAssembly linear memories to
efficiently allocate/grow/deallocate.

OS support at this time primarily includes Windows, macOS, and Linux. Other
OSes such as iOS, Android, and Illumos are supported but less well tested.
PRs to the Wasmtime repository are welcome for new OSes for better native
platform support of a runtime environment.

## Support for `#![no_std]`

The `wasmtime` crate supports being build on no\_std platforms in Rust, but
only for a subset of its compile-time Cargo features. Currently supported
Cargo features are:

* `runtime`
* `gc`
* `component-model`
* `pulley`

This notably does not include the `default` feature which means that when
depending on Wasmtime you'll need to specify `default-features = false`. This
also notably does not include Cranelift or Winch at this time meaning that
no\_std platforms must be used in AOT mode where the module is precompiled
elsewhere.

Wasmtime's support for no\_std requires the embedder to implement the equivalent
of a C header file to indicate how to perform basic OS operations such as
allocating virtual memory. This API can be found as `wasmtime-platform.h` in
Wasmtime's release artifacts or at
`examples/min-platform/embedding/wasmtime-platform.h` in the source tree. Note
that this API is not guaranteed to be stable at this time, it'll need to be
updated when Wasmtime is updated.

Wasmtime's runtime will use the symbols defined in this file meaning that if
they're not defined then a link-time error will be generated. Embedders are
required to implement these functions in accordance with their documentation to
enable Wasmtime to run on custom platforms.

Note that many functions in this header file are gated behind off-by-default
`#ifdef` directives indicating that Wasmtime doesn't require them by default.
The `wasmtime` crate features `custom-{virtual-memory,native-signals}` can be
used to enable usage of these APIs if desired.
