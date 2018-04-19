=======================
Cretonne Code Generator
=======================

Cretonne is a low-level retargetable code generator. It translates a `target-independent
intermediate representation <https://cretonne.readthedocs.io/en/latest/langref.html>`_ into executable
machine code.

.. image:: https://readthedocs.org/projects/cretonne/badge/?version=latest
    :target: https://cretonne.readthedocs.io/en/latest/?badge=latest
    :alt: Documentation Status

.. image:: https://travis-ci.org/cretonne/cretonne.svg?branch=master
    :target: https://travis-ci.org/cretonne/cretonne
    :alt: Build Status

.. image:: https://badges.gitter.im/cretonne/cretonne.png
    :target: https://gitter.im/cretonne/Lobby/~chat
    :alt: Gitter chat

For more information, see `the documentation
<https://cretonne.readthedocs.io/en/latest/?badge=latest>`_.

Status
------

Cretonne currently supports enough functionality to run a wide variety of
programs, including all the functionality needed to execute WebAssembly MVP
functions, although it needs to be used within an external WebAssembly
embedding to be part of a complete WebAssembly implementation.

The x86-64 backend is currently the most complete and stable; other
architectures are in various stages of development. Cretonne currently supports
the System V AMD64 ABI calling convention used on many platforms, but does not
yet support the Windows x64 calling convention. The performance of code
produced by Cretonne is not yet impressive, though we have plans to fix that.

The core codegen crates have minimal dependencies, support
`no_std <#building-with-no-std>` mode, and do not require any host
floating-point support.

Cretonne does not yet perform mitigations for Spectre or related security
issues, though it may do so in the future. It does not currently make any
security-relevant instruction timing guarantees. It has seen a fair amount
of testing and fuzzing, although more work is needed before it would be
ready for a production use case.

Cretonne's APIs are not yet stable.

Cretonne currently supports Rust 1.22.1 and later. We intend to always support
the latest *stable* Rust. And, we currently support the version of Rust in the
latest Ubuntu LTS, although whether we will always do so is not yet determined.
Cretonne requires Python 2.7 or Python 3 to build.

Planned uses
------------

Cretonne is designed to be a code generator for WebAssembly, but it is general enough to be useful
elsewhere too. The initial planned uses that affected its design are:

1. `WebAssembly compiler for the SpiderMonkey engine in Firefox
   <spidermonkey.rst#phase-1-webassembly>`_.
2. `Backend for the IonMonkey JavaScript JIT compiler in Firefox
   <spidermonkey.rst#phase-2-ionmonkey>`_.
3. `Debug build backend for the Rust compiler <rustc.rst>`_.

Building Cretonne
-----------------

Cretonne is using the Cargo package manager format. First, ensure you have
installed a current stable rust (stable, beta, and nightly should all work, but
only stable and beta are tested consistently). Then, change the working
directory to your clone of cretonne and run::

    cargo build

This will create a *target/debug* directory where you can find the generated
binary.

To build the optimized binary for release::

    cargo build --release

You can then run tests with::

    ./test-all.sh

You may need to install the *wat2wasm* tool from the `wabt
<https://github.com/WebAssembly/wabt>`_ project in order to run all of the
WebAssembly tests. Tests requiring wat2wasm are ignored if the tool is not
installed.

Building with `no_std`
----------------------

The following crates support `no_std`:
 - `cretonne-entity`
 - `cretonne-codegen`
 - `cretonne-frontend`
 - `cretonne-native`
 - `cretonne-wasm`
 - `cretonne-module`
 - `cretonne-simplejit`
 - `cretonne`

To use `no_std` mode, disable the `std` feature and enable the `core` feature.
This currently requires nightly rust.

For example, to build `cretonne-codegen`:

.. code-block:: sh

    cd lib/codegen
    cargo build --no-default-features --features core

Or, when using `cretonne-codegen` as a dependency (in Cargo.toml):

.. code-block::

    [dependency.cretonne-codegen]
    ...
    default-features = false
    features = ["core"]

`no_std` support is currently "best effort". We won't try to break it, and
we'll accept patches fixing problems, however we don't expect all developers to
build and test `no_std` when submitting patches. Accordingly, the
`./test-all.sh` script does not test `no_std`.

There is a separate `./test-no_std.sh` script that tests the `no_std`
support in packages which support it.

It's important to note that cretonne still needs liballoc to compile.
Thus, whatever environment is used must implement an allocator.

Also, to allow the use of HashMaps with `no_std`, an external crate called
`hashmap_core` is pulled in (via the `core` feature). This is mostly the same
as `std::collections::HashMap`, except that it doesn't have DOS protection.
Just something to think about.

Building the documentation
--------------------------

To build the Cretonne documentation, you need the `Sphinx documentation
generator <https://www.sphinx-doc.org/>`_::

    $ pip install sphinx sphinx-autobuild sphinx_rtd_theme
    $ cd cretonne/docs
    $ make html
    $ open _build/html/index.html

We don't support Sphinx versions before 1.4 since the format of index tuples
has changed.
