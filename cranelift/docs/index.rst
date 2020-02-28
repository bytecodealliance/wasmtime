Cranelift Code Generator
========================

Contents:

.. toctree::
   :maxdepth: 1

   ir
   meta
   testing
   regalloc
   compare-llvm

Rust Crate Documentation
========================

`cranelift <https://docs.rs/cranelift-codegen/>`_
    This is an umbrella crate that re-exports the codegen and frontend crates,
    to make them easier to use.

`cranelift-codegen <https://docs.rs/cranelift-codegen/>`_
    This is the core code generator crate. It takes Cranelift IR as input
    and emits encoded machine instructions, along with symbolic relocations,
    as output.

`cranelift-codegen-meta <https://docs.rs/cranelift-codegen-meta/>`_
    This crate contains the meta-language utilities and descriptions used by the
    code generator.

`cranelift-wasm <https://docs.rs/cranelift-wasm/>`_
    This crate translates WebAssembly code into Cranelift IR.

`cranelift-frontend <https://docs.rs/cranelift-frontend/>`_
    This crate provides utilities for translating code into Cranelift IR.

`cranelift-native <https://docs.rs/cranelift-native/>`_
    This crate performs auto-detection of the host, allowing Cranelift to
    generate code optimized for the machine it's running on.

`cranelift-reader <https://docs.rs/cranelift-reader/>`_
    This crate translates from Cranelift IR's text format into Cranelift IR
    in in-memory data structures.

`cranelift-module <https://docs.rs/cranelift-module/>`_
    This crate manages compiling multiple functions and data objects
    together.

`cranelift-object <https://docs.rs/cranelift-object/>`_
    This crate provides a object-based backend for `cranelift-module`, which
    emits native object files using the
    `object <https://github.com/gimli-rs/object>`_ library.

`cranelift-faerie <https://docs.rs/cranelift-faerie/>`_
    This crate provides a faerie-based backend for `cranelift-module`, which
    emits native object files using the
    `faerie <https://github.com/m4b/faerie>`_ library.

`cranelift-simplejit <https://docs.rs/cranelift-simplejit/>`_
    This crate provides a simple JIT backend for `cranelift-module`, which
    emits code and data into memory.

Indices and tables
==================

* :ref:`genindex`
* :ref:`modindex`
* :ref:`search`

Todo list
=========

.. todolist::
