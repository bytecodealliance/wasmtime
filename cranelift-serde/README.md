This crate performs serialization of the [Cranelift](https://crates.io/crates/cranelift) IR.

This crate is structured as an optional ability to serialize and deserialize cranelift IR into JSON
format.

Status
------

Cranelift IR can be serialized into JSON.

Deserialize is a work in progress, as it currently deserializes into the serializable data structure
that can be utilized by serde instead of the actual Cranelift IR data structure.


Building and Using Cranelift Serde
----------------------------------

clif-json usage:

    clif-json serialize [-p] <file>
    clif-json deserialize <file>

Where the -p flag outputs Cranelift IR as pretty JSON.

For example to build and use clif-json:

``` {.sourceCode .sh}
cd cranelift-serde
cargo build
clif-json serialize -p test.clif
```

