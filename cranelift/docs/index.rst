Cretonne Code Generator
=======================

Contents:

.. toctree::
   :maxdepth: 1

   langref
   metaref
   testing
   regalloc
   compare-llvm

Rust Crate Documentation
========================

`cretonne <https://docs.rs/cretonne/>`_
    This is the core code generator crate. It takes Cretonne IR as input
    and emits encoded machine instructions, along with symbolic relocations,
    as output.

`cretonne-wasm <https://docs.rs/cretonne-wasm/>`_
    This crate translates WebAssembly code into Cretonne IR.

`cretonne-frontend <https://docs.rs/cretonne-frontend/>`_
    This crate provides utilities for translating code into Cretonne IR.

`cretonne-native <https://docs.rs/cretonne-native/>`_
    This crate performs auto-detection of the host, allowing Cretonne to
    generate code optimized for the machine it's running on.

`cretonne-reader <https://docs.rs/cretonne-reader/>`_
    This crate translates from Cretonne IR's text format into Cretonne IR
    in in-memory data structures.

`cretonne-module <https://docs.rs/cretonne-module/>`_
    This crate manages compiling multiple functions and data objects
    together.

`cretonne-faerie <https://docs.rs/cretonne-faerie/>`_
    This crate provides a faerie-based backend for `cretonne-module`, which
    emits native object files using the
    `faerie <https://crates.io/crates/faerie/>`_ library.

`cretonne-simplejit <https://docs.rs/cretonne-simplejit/>`_
    This crate provides a simple JIT backend for `cretonne-module`, which
    emits code and data into memory.

Indices and tables
==================

* :ref:`genindex`
* :ref:`modindex`
* :ref:`search`

Todo list
=========

.. todolist::
