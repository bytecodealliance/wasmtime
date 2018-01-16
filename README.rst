=======================
Cretonne Code Generator
=======================

Cretonne is a low-level retargetable code generator. It translates a `target-independent
intermediate language <http://cretonne.readthedocs.io/en/latest/langref.html>`_ into executable
machine code.

*This is a work in progress that is not yet functional.*

.. image:: https://readthedocs.org/projects/cretonne/badge/?version=latest
    :target: https://cretonne.readthedocs.io/en/latest/?badge=latest
    :alt: Documentation Status

.. image:: https://travis-ci.org/Cretonne/cretonne.svg?branch=master
    :target: https://travis-ci.org/Cretonne/cretonne
    :alt: Build Status

For more information, see `the documentation
<https://cretonne.readthedocs.io/en/latest/?badge=latest>`_.

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

To build cretonne without libstd, enable the `no_std` feature on `lib/cretonne`,
`lib/frontend`, `lib/native`, and `lib/wasm`.

For example, to build `cretonne`:

    cd lib/cretonne
    cargo build --features no_std

Or, when using `cretonne` as a dependency (in Cargo.toml):

    [dependency.cretonne]
    path = "..."
    features = ["no_std"]

Currently, tests don't test the `no_std` feature:

1. `cargo test --features no_std` won't compile.

1. `./test-all.sh` doesn't test the `no_std` feature.

It's important to note that cretonne still needs liballoc to compile.
Thus, whatever environment is used must implement an allocator.

Also, to allow the use of HashMaps in `no_std` mode, an external crate
called `hashmap_core` is pulled in (only in `no_std` builds). This
is mostly the same as `std::collections::HashMap`, except that it doesn't
have DOS protection. Just something to think about.

Lastly, to support `std::error`, which isn't is `std` or `alloc` for
an inexplicable reason, the `error_core` crate is also used in `no_std` builds.
You might need it, as well, when interfacing with `CtonError`.

Building the documentation
--------------------------

To build the Cretonne documentation, you need the `Sphinx documentation
generator <http://www.sphinx-doc.org/>`_::

    $ pip install sphinx sphinx-autobuild sphinx_rtd_theme
    $ cd cretonne/docs
    $ make html
    $ open _build/html/index.html

We don't support Sphinx versions before 1.4 since the format of index tuples
has changed.
