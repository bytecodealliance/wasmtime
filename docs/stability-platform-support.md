# Platform Support

This page is intended to give a high-level overview of Wasmtime's platform
support along with some aspirations of Wasmtime. For more details see the
documentation on [tiers of stability](./stability-tiers.md) which has specific
information about what's supported in Wasmtime on a per-matrix-combination
basis.

Wasmtime strives to support hardware that anyone wants to run WebAssembly on.
Maintainers of Wasmtime support a number of "major" platforms themselves but
porting work may be required to support platforms that maintainers are not
themselves familiar with. Out-of-the box Wasmtime supports:

* Linux x86\_64, aarch64, s390x, and riscv64
* macOS x86\_64, aarch64
* Windows x86\_64

Other platforms such as Android, iOS, and the BSD family of OSes are not
built-in yet. PRs for porting are welcome and maintainers are happy to add more
entries to the CI matrix for these platforms.

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

At this time `wasmtime` does not have a mode in which it simply interprets
WebAssembly code. It is desired to add support for an interpreter, however, and
this will have minimal system dependencies. It is planned that the system will
need to support some form of dynamic memory allocation, but other than that not
much else will be needed.

## Support for `#![no_std]`

The `wasmtime` crate supports being build on no\_std platforms in Rust, but
only for a subset of its compile-time Cargo features. Currently supported
Cargo features are:

* `runtime`
* `gc`
* `component-model`

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
