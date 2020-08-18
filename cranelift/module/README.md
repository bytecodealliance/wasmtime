This crate provides module-level functionality, which allow multiple
functions and data to be emitted with
[Cranelift](https://crates.io/crates/cranelift) and then linked together.

This crate is structured as an optional layer on top of cranelift-codegen.
It provides additional functionality, such as linking, however users that
require greater flexibility don't need to use it.

A `Module` is a collection of functions and data objects that are linked
together. `Backend` is a trait that defines an interface for backends
that compile modules into various forms. Most users will use one of the
following `Backend` implementations:

 - `SimpleJITBackend`, provided by [cranelift-simplejit], which JITs
   code to memory for direct execution.
 - `ObjectBackend`, provided by [cranelift-object], which emits native
   object files.

[cranelift-simplejit]: https://crates.io/crates/cranelift-simplejit
[cranelift-object]: https://crates.io/crates/cranelift-object
