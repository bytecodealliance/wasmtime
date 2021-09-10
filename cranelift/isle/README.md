# ISLE: Instruction Selection/Lowering Expressions DSL

ISLE is a DSL that allows one to write instruction-lowering rules for a
compiler backend. It is based on a "term-rewriting" paradigm in which the input
-- some sort of compiler IR -- is, conceptually, a tree of terms, and we have a
set of rewrite rules that turn this into another tree of terms.

This repository contains a prototype meta-compiler that compiles ISLE rules
down to an instruction selector implementation in generated Rust code. The
generated code operates efficiently in a single pass over the input, and merges
all rules into a decision tree, sharing work where possible, while respecting
user-configurable priorities on each rule.

The ISLE language is designed so that the rules can both be compiled into an
efficient compiler backend and can be used in formal reasoning about the
compiler. The compiler in this repository implements the former. The latter
use-case is future work and outside the scope of this prototype, but at a high
level, the rules can be seen as simple equivalences between values in two
languages, and so should be translatable to formal constraints or other logical
specification languages.

Some more details are in [BA RFC
#15](https://github.com/bytecodealliance/rfcs/pull/15); additional
documentation will eventually be added to carefully specify the language
semantics.

## Sketch of Instruction Selector

Please see [this Cranelift
branch](https://github.com/cfallin/wasmtime/tree/isle) for an ongoing sketch of
an instruction selector backend in Cranelift that uses ISLE.

## Example Usage

```plain
    $ cargo build --release
    $ target/release/isle -i isle_examples/test.isle -o isle_examples/test.rs
    $ rustc isle_examples/test_main.rs
```
