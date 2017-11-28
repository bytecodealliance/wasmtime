=======================
Cretonne Code Generator
=======================

Cretonne is a low-level retargetable code generator. It translates a
target-independent intermediate language into executable machine code.

*This is a work in progress that is not yet functional.*

.. image:: https://readthedocs.org/projects/cretonne/badge/?version=latest
    :target: https://cretonne.readthedocs.io/en/latest/?badge=latest
    :alt: Documentation Status

.. image:: https://travis-ci.org/stoklund/cretonne.svg?branch=master
    :target: https://travis-ci.org/stoklund/cretonne
    :alt: Build Status

Cretonne is designed to be a code generator for WebAssembly with these design
goals:

Portable semantics
    As far as possible, Cretonne's input language has well-defined semantics
    that are the same on all target architectures. The semantics are usually
    the same as WebAssembly's.
Fast sandbox verification
    Cretonne's input language has a safe subset for sandboxed code. No advanced
    analysis is required to verify memory safety as long as only the safe
    subset is used. The safe subset is expressive enough to implement
    WebAssembly.
Scalable performance
    Cretonne can be configured to generate code as quickly as possible, or it
    can generate very good code at the cost of slower compile times.
Predictable performance
    When optimizing, Cretonne focuses on adapting the target-independent IL to
    the quirks of the target architecture. There are no advanced optimizations
    that sometimes work, sometimes fail.

For more information, see
`the documentation <https://cretonne.readthedocs.io/en/latest/?badge=latest>`_.

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
