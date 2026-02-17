# Using the Wasmtime API

Wasmtime can be used as a library to embed WebAssembly execution support
within applications. Wasmtime is written in Rust, but bindings are available
through a C API for a number of other languages too. This chapter has a number
of examples which come from the Wasmtime repository itself and showcase the
Rust, C, and C++ embedding APIs.

# Officially supported bindings

The following languages are all developed in the Wasmtime repository itself and
have tests/documentation in the main repository itself.

## Rust

Wasmtime is itself written in Rust and is available as the [`wasmtime`] crate on
crates.io. API reference documentation for Wasmtime can be found on
[docs.rs](https://docs.rs/wasmtime) and includes a number of examples throughout
the documentation.

[`wasmtime`]: https://crates.io/crates/wasmtime

## C

Wasmtime provides a C API through `libwasmtime.a`, for example. The C API is
developed/tested in the Wasmtime repository itself and is the main entrypoint of
all other language support for Wasmtime other than Rust. Note that the C API is
not always in perfect parity with the Rust API and can lag behind in terms of
features. This is not intentional, however, and with sufficient development
resources the two APIs will be kept in-sync.

Documentation for the C API can be found at
[docs.wasmtime.dev/c-api](https://docs.wasmtime.dev/c-api/). Documentation on
how to build the C API can be found in the [README of the C API]. Building the C
API uses CMake to orchestrate the build and Cargo under the hood to build the
Rust code.

The C API can also be installed through `*-c-api-*` [release artifacts].

[README of the C API]: https://github.com/bytecodealliance/wasmtime/blob/main/crates/c-api/README.md
[release artifacts]: https://github.com/bytecodealliance/wasmtime/releases/latest

## C++

Wasmtime supports C++ as a header-based library layered on top of the C API. The
C++ API header use the `*.hh` extension and are installed alongside the C API
meaning that if you install the C API you've got the C++ API as well. The C++
API is focused on automating resource management of the C API and providing an
easier-to-use API on top.

## Bash

Wasmtime is available in Bash through the [`wasmtime` CLI](./cli.md) executable
and its various subcommands. Source for the `wasmtime` executable is developed
in the same repository as Wasmtime itself. Installation of `wasmtime` can be
[found in this documentation](./cli-install.md).

# External bindings to Wasmtime

The following language bindings are all developed outside of the Wasmtime
repository. Consequently they are not officially supported and may have varying
levels of support and activity. Note that many of these bindings are in the
bytecodealliance GitHub organization but are still not tested/developed in-sync
with the main repository.

## Python

Python bindings for Wasmtime are developed in the [wasmtime-py
repository](https://github.com/bytecodealliance/wasmtime-py). These bindings are
built on the C API and developed externally from the main Wasmtime repository so
updates can lag behind the main repository sometimes in terms of release
schedule and features.

Python bindings are published to the
[`wasmtime`](https://pypi.org/project/wasmtime/) package on PyPI.

## Go

Go bindings for Wasmtime are developed in the [wasmtime-go
repository](https://github.com/bytecodealliance/wasmtime-go). These bindings are
built on the C API and developed externally from the main Wasmtime repository so
updates can lag behind the main repository sometimes in terms of release
schedule and features.

Documentation for the Go API bindings can be found [on
pkg.go.dev](https://pkg.go.dev/github.com/bytecodealliance/wasmtime-go), and be
sure to use the version-picker to pick the latest major version which tracks
Wasmtime's own major versions.

## .NET

The [Wasmtime](https://www.nuget.org/packages/Wasmtime) NuGet package can be
used to programmatically interact with WebAssembly modules and requires
[.NET Core SDK 3.0 SDK or later](https://dotnet.microsoft.com/download)
installed as well.

The [.NET embedding of Wasmtime
repository](https://github.com/bytecodealliance/wasmtime-dotnet) contains the
source code for the Wasmtime NuGet package and
the repository also has more
[examples](https://github.com/bytecodealliance/wasmtime-dotnet/tree/main/examples)
as well.

## Ruby

Wasmtime [is available on RubyGems](https://rubygems.org/gems/wasmtime) and can
be used programmatically to interact with Wasm modules. To learn more, check out
the [more advanced
examples](https://github.com/bytecodealliance/wasmtime-rb/tree/main/examples)
and the [API
documentation](https://bytecodealliance.github.io/wasmtime-rb/latest/). If you
have any questions, do not hesitate to open an issue on the [GitHub
repository](https://github.com/bytecodealliance/wasmtime-rb).

## Elixir

Wasmtime [is available on Hex](https://hex.pm/packages/wasmex) and can be used
programmatically to interact with Wasm modules. To learn more, check out an
[another example](https://github.com/tessi/wasmex#example) and the [API
documentation](https://hexdocs.pm/wasmex/Wasmex.html).  If you have any
questions, do not hesitate to open an issue on the [GitHub
repository](https://github.com/tessi/wasmex).
