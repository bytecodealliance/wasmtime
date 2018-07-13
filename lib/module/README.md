This crate provides the `Module` trait, which provides an interface for
multiple functions and data to be emitted with
[Cranelift](https://crates.io/crates/cranelift) and then linked together.

This crate is structured as an optional layer on top of cranelift-codegen.
It provides additional functionality, such as linking, however users that
require greater flexibility don't need to use it.
