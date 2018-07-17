Cranelift Code Generator
========================

Cranelift is a low-level retargetable code generator. It translates a
[target-independent intermediate
representation](https://cranelift.readthedocs.io/en/latest/langref.html)
into executable machine code.

[![Documentation Status](https://readthedocs.org/projects/cranelift/badge/?version=latest)](https://cranelift.readthedocs.io/en/latest/?badge=latest)
[![Build Status](https://travis-ci.org/CraneStation/cranelift.svg?branch=master)](https://travis-ci.org/CraneStation/cranelift)
[![Gitter chat](https://badges.gitter.im/CraneStation/CraneStation.svg)](https://gitter.im/CraneStation/Lobby/~chat)

For more information, see [the
documentation](https://cranelift.readthedocs.io/en/latest/?badge=latest).

Status
------

Cranelift currently supports enough functionality to run a wide variety
of programs, including all the functionality needed to execute
WebAssembly MVP functions, although it needs to be used within an
external WebAssembly embedding to be part of a complete WebAssembly
implementation.

The x86-64 backend is currently the most complete and stable; other
architectures are in various stages of development. Cranelift currently
supports the System V AMD64 ABI calling convention used on many
platforms, but does not yet support the Windows x64 calling convention.
The performance of code produced by Cranelift is not yet impressive,
though we have plans to fix that.

The core codegen crates have minimal dependencies, support
[no\_std](#building-with-no-std) mode, and do not require any host
floating-point support.

Cranelift does not yet perform mitigations for Spectre or related
security issues, though it may do so in the future. It does not
currently make any security-relevant instruction timing guarantees. It
has seen a fair amount of testing and fuzzing, although more work is
needed before it would be ready for a production use case.

Cranelift's APIs are not yet stable.

Cranelift currently supports Rust 1.22.1 and later. We intend to always
support the latest *stable* Rust. And, we currently support the version
of Rust in the latest Ubuntu LTS, although whether we will always do so
is not yet determined. Cranelift requires Python 2.7 or Python 3 to
build.

Planned uses
------------

Cranelift is designed to be a code generator for WebAssembly, but it is
general enough to be useful elsewhere too. The initial planned uses that
affected its design are:

 - [WebAssembly compiler for the SpiderMonkey engine in
    Firefox](spidermonkey.md#phase-1-webassembly).
 - [Backend for the IonMonkey JavaScript JIT compiler in
    Firefox](spidermonkey.md#phase-2-ionmonkey).
 - [Debug build backend for the Rust compiler](rustc.md).

Building Cranelift
------------------

Cranelift uses a [conventional Cargo build
process](https://doc.rust-lang.org/cargo/guide/working-on-an-existing-project.html).

Cranelift consists of a collection of crates, and uses a [Cargo
Workspace](https://doc.rust-lang.org/book/second-edition/ch14-03-cargo-workspaces.html),
so for some cargo commands, such as `cargo test`, the `--all` is needed
to tell cargo to visit all of the crates.

`test-all.sh` at the top level is a script which runs all the cargo
tests and also performs code format, lint, and documentation checks.

Building with no\_std
---------------------

The following crates support \`no\_std\`:
 - cranelift-entity
 - cranelift-codegen
 - cranelift-frontend
 - cranelift-native
 - cranelift-wasm
 - cranelift-module
 - cranelift-simplejit
 - cranelift

To use no\_std mode, disable the std feature and enable the core
feature. This currently requires nightly rust.

For example, to build \`cranelift-codegen\`:

``` {.sourceCode .sh}
cd lib/codegen
cargo build --no-default-features --features core
```

Or, when using cranelift-codegen as a dependency (in Cargo.toml):

``` {.sourceCode .}
[dependency.cranelift-codegen]
...
default-features = false
features = ["core"]
```

no\_std support is currently "best effort". We won't try to break it,
and we'll accept patches fixing problems, however we don't expect all
developers to build and test no\_std when submitting patches.
Accordingly, the ./test-all.sh script does not test no\_std.

There is a separate ./test-no\_std.sh script that tests the no\_std
support in packages which support it.

It's important to note that cranelift still needs liballoc to compile.
Thus, whatever environment is used must implement an allocator.

Also, to allow the use of HashMaps with no\_std, an external crate
called hashmap\_core is pulled in (via the core feature). This is mostly
the same as std::collections::HashMap, except that it doesn't have DOS
protection. Just something to think about.

Building the documentation
--------------------------

To build the Cranelift documentation, you need the [Sphinx documentation
generator](https://www.sphinx-doc.org/):

    $ pip install sphinx sphinx-autobuild sphinx_rtd_theme
    $ cd cranelift/docs
    $ make html
    $ open _build/html/index.html

We don't support Sphinx versions before 1.4 since the format of index
tuples has changed.
