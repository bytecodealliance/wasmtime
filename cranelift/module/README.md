This crate provides module-level functionality, which allow multiple
functions and data to be emitted with
[Cranelift](https://crates.io/crates/cranelift) and then linked together.

This crate is structured as an optional layer on top of cranelift-codegen.
It provides additional functionality, such as linking, however users that
require greater flexibility don't need to use it.

A module is a collection of functions and data objects that are linked
together. The `Module` trait that defines a common interface for various kinds
of modules. Most users will use one of the following `Module` implementations:

 - `JITModule`, provided by [cranelift-jit], which JITs code to memory for direct execution.
 - `ObjectModule`, provided by [cranelift-object], which emits native object files.

[cranelift-jit]: https://crates.io/crates/cranelift-jit
[cranelift-object]: https://crates.io/crates/cranelift-object
