This crate provides a straightforward way to create a
[Cretonne](https://crates.io/crates/cretonne) IR function and fill it with
instructions translated from another language. It contains an SSA construction
module that provides convenient methods for translating non-SSA variables into
SSA Cretonne IR values via `use_var` and `def_var` calls.
