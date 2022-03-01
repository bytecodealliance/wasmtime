# ISLE: Instruction Selection/Lowering Expressions DSL

See also: [Language Reference](./docs/language-reference.md)

## Table of Contents

* [Introduction](#introduction)
* [Example Usage](#example-usage)
* [Tutorial](#tutorial)
* [Implementation](#implementation)

## Introduction

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

Some more details and motivation are in [BA RFC #15](https://github.com/bytecodealliance/rfcs/pull/15).
Reference documentation can be found [here](docs/language-reference.md).
Details on ISLE's integration into Cranelift can be found
[here](../docs/isle-integration.md).

## Example Usage

Build `islec`, the ISLE compiler:

```shell
$ cargo build --release
```

Compile a `.isle` source file into Rust code:

```shell
$ target/release/islec -i isle_examples/test.isle -o isle_examples/test.rs
```

Include that Rust code in your crate and compile it:

```shell
$ rustc isle_examples/test_main.rs
```

## Tutorial

This tutorial walks through defining an instruction selection and lowering pass
for a simple, RISC-y, high-level IR down to low-level, CISC-y machine
instructions. It is intentionally somewhat similar to CLIF to MachInst lowering,
although it restricts the input and output languages to only adds, loads, and
constants so that we can focus on ISLE itself.

> The full ISLE source code for this tutorial is available at
> `isle_examples/tutorial.isle`.

The ISLE language is based around rules for translating a term (i.e. expression)
into another term. Terms are typed, so before we can write rules for translating
some type of term into another type of term, we have to define those types:

```lisp
;; Declare that we are using the `i32` primitive type from Rust.
(type i32 (primitive i32))

;; Our high-level, RISC-y input IR.
(type HighLevelInst
  (enum (Add (a Value) (b Value))
        (Load (addr Value))
        (Const (c i32))))

;; A value in our high-level IR is a Rust `Copy` type. Values are either defined
;; by an instruction, or are a basic block argument.
(type Value (primitive Value))

;; Our low-level, CISC-y machine instructions.
(type LowLevelInst
  (enum (Add (mode AddrMode))
        (Load (offset i32) (addr Reg))
        (Const (c i32))))

;; Different kinds of addressing modes for operands to our low-level machine
;; instructions.
(type AddrMode
  (enum
    ;; Both operands in registers.
    (RegReg (a Reg) (b Reg))
    ;; The destination/first operand is a register; the second operand is in
    ;; memory at `[b + offset]`.
    (RegMem (a Reg) (b Reg) (offset i32))
    ;; The destination/first operand is a register, second operand is an
    ;; immediate.
    (RegImm (a Reg) (imm i32))))

;; The register type is a Rust `Copy` type.
(type Reg (primitive Reg))
```

Now we can start writing some basic lowering rules! We declare the top-level
lowering function (a "constructor term" in ISLE terminology) and attach rules to
it. The simplest case is matching a high-level `Const` instruction and lowering
that to a low-level `Const` instruction, since there isn't any translation we
really have to do.

```lisp
;; Declare our top-level lowering function. We will attach rules to this
;; declaration for lowering various patterns of `HighLevelInst` inputs.
(decl lower (HighLevelInst) LowLevelInst)

;; Simple rule for lowering constants.
(rule (lower (HighLevelInst.Const c))
      (LowLevelInst.Const c))
```

Each rule has the form `(rule <left-hand side> <right-hand-side>)`. The
left-hand side (LHS) is a *pattern* and the right-hand side (RHS) is an
*expression*. When the LHS pattern matches the input, then we evaluate the RHS
expression. The LHS pattern can bind variables from the input that are then
available in the right-hand side. For example, in our `Const`-lowering rule, the
variable `c` is bound from the LHS and then reused in the RHS.

Now we can compile this code by running

```shell
$ islec isle_examples/tutorial.isle
```

and we'll get the following output <sup>(ignoring any minor code generation
changes in the future)</sup>:

```rust
// GENERATED BY ISLE. DO NOT EDIT!
//
// Generated automatically from the instruction-selection DSL code in:
// - isle_examples/tutorial.isle

// [Type and `Context` definitions removed for brevity...]

// Generated as internal constructor for term lower.
pub fn constructor_lower<C: Context>(ctx: &mut C, arg0: &HighLevelInst) -> Option<LowLevelInst> {
    let pattern0_0 = arg0;
    if let &HighLevelInst::Const { c: pattern1_0 } = pattern0_0 {
        // Rule at isle_examples/tutorial.isle line 45.
        let expr0_0 = LowLevelInst::Const {
            c: pattern1_0,
        };
        return Some(expr0_0);
    }
    return None;
}
```

There are a few things to notice about this generated Rust code:

* The `lower` constructor term becomes the `constructor_lower` function in the
  generated code.

* The function returns a value of type `Option<LowLevelInst>` and returns `None`
  when it doesn't know how to lower an input `HighLevelInst`. This is useful for
  incrementally porting hand-written lowering code to ISLE.

* There is a helpful comment documenting where in the ISLE source code a rule
  was defined. The goal is to ISLE more transparent and less magical.

* The code is parameterized by a type that implements a `Context`
  trait. Implementing this trait is how you glue the generated code into your
  compiler. Right now this is an empty trait; more on `Context` later.

* Lastly, and most importantly, this generated Rust code is basically what we
  would have written by hand to do the same thing, other than things like
  variable names. It checks if the input is a `Const`, and if so, translates it
  into a `LowLevelInst::Const`.

Okay, one rule isn't very impressive, but in order to start writing more rules
we need to be able to put the result of a lowered instruction into a `Reg`. This
might internally have to do arbitrary things like update use counts or anything
else that Cranelift's existing `LowerCtx::put_input_in_reg` does for different
target architectures. To allow for plugging in this kind of arbitrary logic,
ISLE supports *external constructors*. These end up as methods of the `Context`
trait in the generated Rust code, and you can implement them however you want
with custom Rust code.

Here is how we declare an external helper to put a value into a register:

```lisp
;; Declare an external constructor that puts a high-level `Value` into a
;; low-level `Reg`.
(decl put_in_reg (Value) Reg)
(extern constructor put_in_reg put_in_reg)
```

If we rerun `islec` on our ISLE source, instead of an empty `Context` trait, now
we will get this trait definition:

```rust
pub trait Context {
    fn put_in_reg(&mut self, arg0: Value) -> (Reg,);
}
```

With the `put_in_reg` helper available, we can define rules for lowering loads
and adds:

```lisp
;; Simple rule for lowering adds.
(rule (lower (HighLevelInst.Add a b))
      (LowLevelInst.Add
        (AddrMode.RegReg (put_in_reg a) (put_in_reg b))))

;; Simple rule for lowering loads.
(rule (lower (HighLevelInst.Load addr))
      (LowLevelInst.Load 0 (put_in_reg addr)))
```

If we compile our ISLE source into Rust code once again, the generated code for
`lower` now looks like this:

```rust
// Generated as internal constructor for term lower.
pub fn constructor_lower<C: Context>(ctx: &mut C, arg0: &HighLevelInst) -> Option<LowLevelInst> {
    let pattern0_0 = arg0;
    match pattern0_0 {
        &HighLevelInst::Const { c: pattern1_0 } => {
            // Rule at isle_examples/tutorial.isle line 45.
            let expr0_0 = LowLevelInst::Const {
                c: pattern1_0,
            };
            return Some(expr0_0);
        }
        &HighLevelInst::Load { addr: pattern1_0 } => {
            // Rule at isle_examples/tutorial.isle line 59.
            let expr0_0: i32 = 0;
            let expr1_0 = C::put_in_reg(ctx, pattern1_0);
            let expr2_0 = LowLevelInst::Load {
                offset: expr0_0,
                addr: expr1_0,
            };
            return Some(expr2_0);
        }
        &HighLevelInst::Add { a: pattern1_0, b: pattern1_1 } => {
            // Rule at isle_examples/tutorial.isle line 54.
            let expr0_0 = C::put_in_reg(ctx, pattern1_0);
            let expr1_0 = C::put_in_reg(ctx, pattern1_1);
            let expr2_0 = AddrMode::RegReg {
                a: expr0_0,
                b: expr1_0,
            };
            let expr3_0 = LowLevelInst::Add {
                mode: expr2_0,
            };
            return Some(expr3_0);
        }
        _ => {}
    }
    return None;
}
```

As you can see, each of our rules was collapsed into a single, efficient `match`
expression. Just like we would have otherwise written by hand. And wherever we
need to get a high-level operand as a low-level register, there is a call to the
`Context::put_in_reg` trait method, allowing us to hook whatever arbitrary logic
we need to when putting a value into a register when we implement the `Context`
trait.

Things start to get more interesting when we want to do things like sink a load
into the add's addressing mode. This is only desirable when our add is the only
use of the loaded value. Furthermore, it is only valid to do when there isn't
any store that might write to the same address we are loading from in between
the load and the add. Otherwise, moving the load across the store could result
in a miscompilation where we load the wrong value to add:

```text
x = load addr
store 42 -> addr
y = add x, 1

==/==>

store 42 -> addr
x = load addr
y = add x, 1
```

We can encode these kinds of preconditions in an *external extractor*. An
extractor is like our regular constructor functions, but it is used inside LHS
patterns, rather than RHS expressions, and its arguments and results flipped
around: instead of taking arguments and producing results, it takes a result and
(fallibly) produces the arguments. This allows us to write custom preconditions
for matching code.

Let's make this more clear with a concrete example. Here is the declaration of
an external extractor to match on the high-level instruction that defined a
given operand `Value`, along with a new rule to sink loads into adds:

```lisp
;; Declare an external extractor for extracting the instruction that defined a
;; given operand value.
(decl inst_result (HighLevelInst) Value)
(extern extractor inst_result inst_result)

;; Rule to sink loads into adds.
(rule (lower (HighLevelInst.Add a (inst_result (HighLevelInst.Load addr))))
      (LowLevelInst.Add
        (AddrMode.RegMem (put_in_reg a)
                         (put_in_reg addr)
                         0)))
```

Note that the operand `Value` passed into this extractor might be a basic block
parameter, in which case there is no such instruction. Or there might be a store
or function call instruction in between the current instruction and the
instruction that defines the given operand value, in which case we want to
"hide" the instruction so that we don't illegally sink loads into adds they
shouldn't be sunk into. So this extractor might fail to return an instruction
for a given operand `Value`.

If we recompile our ISLE source into Rust code once again, we see a new
`inst_result` method defined on our `Context` trait, we notice that its
arguments and returns are flipped around from the `decl` in the ISLE source
because it is an extractor, and finally that it returns an `Option` because it
isn't guaranteed that we can extract a defining instruction for the given
operand `Value`:

```rust
pub trait Context {
    fn put_in_reg(&mut self, arg0: Value) -> (Reg,);
    fn inst_result(&mut self, arg0: Value) -> Option<(HighLevelInst,)>;
}
```

And if we look at the generated code for our `lower` function, there is a new,
nested case for sinking loads into adds that uses the `Context::inst_result`
trait method to see if our new rule can be applied:

```rust
// Generated as internal constructor for term lower.
pub fn constructor_lower<C: Context>(ctx: &mut C, arg0: &HighLevelInst) -> Option<LowLevelInst> {
    let pattern0_0 = arg0;
    match pattern0_0 {
        &HighLevelInst::Const { c: pattern1_0 } => {
            // [...]
        }
        &HighLevelInst::Load { addr: pattern1_0 } => {
            // [...]
        }
        &HighLevelInst::Add { a: pattern1_0, b: pattern1_1 } => {
            if let Some((pattern2_0,)) = C::inst_result(ctx, pattern1_1) {
                if let &HighLevelInst::Load { addr: pattern3_0 } = &pattern2_0 {
                    // Rule at isle_examples/tutorial.isle line 68.
                    let expr0_0 = C::put_in_reg(ctx, pattern1_0);
                    let expr1_0 = C::put_in_reg(ctx, pattern3_0);
                    let expr2_0: i32 = 0;
                    let expr3_0 = AddrMode::RegMem {
                        a: expr0_0,
                        b: expr1_0,
                        offset: expr2_0,
                    };
                    let expr4_0 = LowLevelInst::Add {
                        mode: expr3_0,
                    };
                    return Some(expr4_0);
                }
            }
            // Rule at isle_examples/tutorial.isle line 54.
            let expr0_0 = C::put_in_reg(ctx, pattern1_0);
            let expr1_0 = C::put_in_reg(ctx, pattern1_1);
            let expr2_0 = AddrMode::RegReg {
                a: expr0_0,
                b: expr1_0,
            };
            let expr3_0 = LowLevelInst::Add {
                mode: expr2_0,
            };
            return Some(expr3_0);
        }
        _ => {}
    }
    return None;
}
```

Once again, this is pretty much the code you would have otherwise written by
hand to sink the load into the add.

At this point we can start defining a whole bunch of even-more-complicated
lowering rules that do things like take advantage of folding static offsets into
loads into adds:

```lisp
;; Rule to sink a load of a base address with a static offset into a single add.
(rule (lower (HighLevelInst.Add
               a
               (inst_result (HighLevelInst.Load
                              (inst_result (HighLevelInst.Add
                                             base
                                             (inst_result (HighLevelInst.Const offset))))))))
      (LowLevelInst.Add
        (AddrMode.RegMem (put_in_reg a)
                         (put_in_reg base)
                         offset)))

;; Rule for sinking an immediate into an add.
(rule (lower (HighLevelInst.Add a (inst_result (HighLevelInst.Const c))))
      (LowLevelInst.Add
        (AddrMode.RegImm (put_in_reg a) c)))

;; Rule for lowering loads of a base address with a static offset.
(rule (lower (HighLevelInst.Load
               (inst_result (HighLevelInst.Add
                              base
                              (inst_result (HighLevelInst.Const offset))))))
      (LowLevelInst.Load offset (put_in_reg base)))
```

I'm not going to show the generated Rust code for these new rules here because
it is starting to get a bit too big. But you can compile
`isle_examples/tutorial.isle` and verify yourself that it generates the code you
expect it to.

In conclusion, adding new lowering rules is easy with ISLE. And you still get
that efficient, compact tree of `match` expressions in the generated Rust code
that you would otherwise write by hand.

## Implementation

This is an overview of `islec`'s passes and data structures:

```text
    +------------------+
    | ISLE Source Text |
    +------------------+
             |
             | Lex
             V
         +--------+
         | Tokens |
         +--------+
             |
             | Parse
             V
   +----------------------+
   | Abstract Syntax Tree |
   +----------------------+
             |
             | Semantic Analysis
             V
+----------------------------+
| Term and Type Environments |
+----------------------------+
             |
             | Trie Construction
             V
       +-----------+
       | Term Trie |
       +-----------+
             |
             | Code Generation
             V
    +------------------+
    | Rust Source Code |
    +------------------+
```

### Lexing

Lexing breaks up the input ISLE source text into a stream of tokens. Our lexer
is pull-based, meaning that we don't eagerly construct the full stream of
tokens. Instead, we wait until the next token is requested, at which point we
lazily lex it.

Relevant source files:

* `isle/src/lexer.rs`

### Parsing

Parsing translates the stream of tokens into an abstract syntax tree (AST). Our
parser is a simple, hand-written, recursive-descent parser.

Relevant source files:

* `isle/src/ast.rs`
* `isle/src/parser.rs`

### Semantic Analysis

Semantic analysis performs type checking, figures out which rules apply to which
terms, etc. It creates a type environment and a term environment that we can use
to get information about our terms throughout the rest of the pipeline.

Relevant source files:

* `isle/src/sema.rs`

### Trie Construction

The trie construction phase linearizes each rule's LHS pattern and inserts them
into a trie that maps LHS patterns to RHS expressions. This trie is the skeleton
of the decision tree that will be emitted during code generation.

Relevant source files:

* `isle/src/ir.rs`
* `isle/src/trie.rs`

### Code Generation

Code generation takes in the term trie and emits Rust source code that
implements it.

Relevant source files:

* `isle/src/codegen.rs`
