# Notes for verifying Cranelift's ISLE rules in SMT

Some exploratory notes for how to structure verification of Cranelift's ISLE lowering rules.
ISLE is a domain-specific language for specifying rewrite rules, primarily for instruction selection.
ISLE source text is [compiled down into Rust
code](https://github.com/bytecodealliance/wasmtime/tree/main/cranelift/isle#implementation).

Documentation on the ISLE-Cranelift integration can be found
[here](./isle-integration.md), and documentation on the language itself can be found
[here](../isle/docs/language-reference.md).

## What do we mean by "verifying" ISLE rules?

At a high level, our goal is to show that ISLE's rules maintain semantic equivalence of program fragments before and after rule application. 
In principal, a fully verified implementation of ISLE would include verification of the rules themselves, the Rust code generation, and the rule application implementation. 
However, as a first step, we are interested in verifying just the first component: the rules themselves. The two reasons I (Alexa) see for starting there:
1. Individual rules are declarative, small, mostly self-contained, and amenable to composable [SMT-style verification](https://github.com/Z3Prover/z3).
2. Prior related work, such as [Alive](https://web.ist.utl.pt/nuno.lopes/pubs/alive-pldi15.pdf), has shown that only looking at rules can still find impactful bugs.

By "verifying" an individual rule, we can probably rely on simple semantic equivalence rather than something more complicated such as refinement, since Cranelift tries to avoid undefined behavior.

The entire set of ISLE rules is designed to take terms from Cranelift's intermediate representation (CLIF) to a MachineInst form that closely matches a particular backend (arm, x86, etc). My current thinking is to first likely focus on arm, then x86. Note, however, that some rules write between intermediate terms rather than being "final" ISA instructions.

In terms of verification, for a rule like:
```lisp
(rule (lower (has_type (fits_in_64 ty) (iadd x y)))
      (value_reg (add ty (put_in_reg x) (put_in_reg y))))
```

We would want to show something like (with made-up syntax and some additional side conditions for types/sizes):
```lisp
(forall 
  (x y)
  (=  (iadd x y)
      (get_reg (put_get (add (put_reg x) (put_reg y))))))
```

In a classic SMT-verification style, we would do this by asserting the negation of the property we want and checking for SAT.
An UNSAT model implies no counterexample is found and the semantic equivalence holds.
We'll likely want to use [SMT's theory of bitvectors](https://smtlib.cs.uiowa.edu/theories-FixedSizeBitVectors.shtml) and focus on integer operations first. 

```lisp
(declare-const x y (_ BitVec 64))
(declare-fun get_reg ...)
(declare-fun put_reg ...)
(assert (let 
    (lhs (bvadd x y))
    (rhs (get_reg (put_get (bvadd (put_reg a) (put_reg b)))))
    (not (=  lhs rhs))))

; If this result is (unsat), the rule holds since no counterexample
; is found. If it returns (sat), the model provides a counterexample.
(check-sat)
```

If we can show this equivalence for every rule, we raise our assurance that ISLE correctly implements instruction lowering for all possible inputs both in terms of the programs Cranelift is compiling and all possible inputs to those programs. 

This basic per-rule strategy should certainly be our "minimum viable product", but there's room for more exciting directions. 
If we run into scaling issues, we can decompose rules or use more nuanced counterexample driven verification (e.g., Counterexample-Guided Abstraction Refinement (CEGAR)).

## Why SMT?

Cranelift is primarily a production engineering project, so our solution should focus on a high degree of automation.
SMT should free engineers from having to construct proofs themselves.
We can also build on existing projects for large-scale instruction set architecture (ISA) semantics that support SMT to handle many "right hand sides" of rules. 

### More motivation

Another exciting component of automated verification is that its flexibility allows engineers to iterate faster on potential new rules.
Instead of needing to manually check or prove a new rule sound, engineers should be able to quickly get feedback from an automated tool.
Even more forward-looking, an automated verification tool opens the door to synthesizing rules themselves in the future.
## Existing ISA semantics

There have been several recent advances in modeling the semantics of real-world ISAs. Our goal should be to build on these as much as possible. 

1. [SAIL semantics, including for arm (POPL 19)](https://www.cl.cam.ac.uk/~pes20/sail/sail-popl2019.pdf)
2. [K-framework semantics for x86 (PLDI 19)](https://dl.acm.org/doi/10.1145/3314221.3314601)
3. [Stoke semantics for x86](https://github.com/StanfordPL/stoke)

However, it's important to note that ISLE rules are not _required_ to compile "all the way" to ISA instructions on the right hand side. Rather, many rules write to temporary values that enable downstream rewrites.

In addition, each of the existing ISA projects had their own slightly different use cases, so we can expect some difficulty massaging them to this use case:
- We need to specify symbolic values not just for registers, but for constant arguments as well, which is likely in a meta-syntax that will differ for each tool. 
- Each tool has its own memory model and helper function for read/writes to registers, and at least for SAIL, the output is a superset of SMTLIB that cannot be fed directly into a solver.

## Example rule

Let's start with a walkthrough of my current favorite rule.

To start, we need to remember that one constraint a good instruction lowering pass cares about is register pressure. 
That is, while intermediate representations can use an unbounded number of named variables, machine code is restricted to a relatively small set of named registers. 
If the same instruction can be implemented with the same latency but use fewer registers, we should do that.

One of the most common ways to lower register use is to use _immediate_ values, which store one or more operands to an instruction in the instruction itself, rather than in an register. Typically, this is only possible when the operand is relatively small (it can fit in some subset of the bits used for full operands).

For example, an arbitrary add `r = a + b` where we don't know anything about the sizes of operands `a` and `b` must be implemented using 3 registers:

    add r x y

If we know `y` is actually some small constant value `C` (say, it fits in 12 bits), we could replace this with:

    add r x C
    add r x 0x01

Even further, if `y` itself uses a lot of bits, but it's _negation_ is small, we can use some clever rearranging to still use a single arithmetic instruction with an immediate:
    
    ; r = x + C == r = x - (-C)
    sub r x (-C) 
    sub r x 0xfe 

Cranelift's ISLE rule to capture this looks like this:
```lisp
;; Same as the previous special cases, except we can switch the addition to a
;; subtraction if the negated immediate fits in 12 bits.
(rule (lower (has_type (fits_in_64 ty) (iadd x (imm12_from_negated_value y))))
      (value_reg (sub_imm ty (put_in_reg x) y)))
```

To break down some components of this rule, starting with the left hand side: 
#### `lower` 
ISLE is typed, and rules must maintain the type of terms. Because CLIF and MachineInst do not share a type, Cranelift uses the `lower` term to indicate that following CLIF is being translated to a different type:
```lisp
(decl lower (Inst) ValueRegs)
```
#### `has_type` 
This is an _internal extractor_ (defined in ISLE itself) that can deconstruct a CLIF term into its constituent parts. Here, it breaks the term into type and instruction.
```lisp
(decl has_type (Type Inst) Inst)
```
#### `fits_in_64`
In contrast, this is an _external extractor_ (defined in arbitrary Rust rather than ISLE) that again breaks apart a term. 
The implementation looks like this:

```rust
fn fits_in_64(&mut self, ty: Type) -> Option<Type> {
    if ty.bits() <= 64 {
        Some(ty)
    } else {
        None
    }
}
```

Here, the function returning an `Option` indicates that the rule should only match on this left hand side if the result is `Some`. 
That is, this piece takes the type extracted from the instruction by `has_type` and extracts the type `ty` if the condition is met and otherwise aborts the match.
#### `iadd`
This is the actual CLIF add instruction.
#### `x`
A vanilla named variable.
#### `imm12_from_negated_value`
Here, we have a bit of a roundabout extractor. `imm12_from_negated_value` itself is internal, but it calls out to an external extractor, `imm12_from_negated_u64`, which again returns conceptually `Some(-C)` if the negation of the right hand operand fits in 12 bits and otherwise aborts.

And now, the right hand side:
#### `value_reg` 
This is an external constructor that consumes a `Reg` and produces a `ValueRegs`, which is one or more registers that collectively represent a value. 
For example, a 128 bit integer would be split into two high- and low-half 64 bit registers but would use the similar `value_regs`.  
As a reminder, `ValueRegs` is the "return type" of the `lower` function on the left hand side, so our left and right hand sides type check.

#### `put_in_reg`
This is an external constructor takes a CLIF value and allocates a temporary register (of type  `Reg`) to hold the value. 
It asserts that the CLIF value will fit in a single register (or dynamically aborts with a panic). 
There is also a similar `put_in_regs` for when the value might require multiple registers (e.g., an i128 value), which returns a `ValueRegs` instead of a single `Reg`.

#### `subimm`
This term is essentially an intermediate term (that is, this rule does not go "all the way"). 
In this case, this allows Cranelift to avoid implementing the same logic for multiple types (`ty`). 
Elsewhere, Cranelift lowers this term for each type:
```lisp
(rule (sub_imm (fits_in_32 _ty) x y) (sub32_imm x y))
(rule (sub_imm $I64 x y) (sub64_imm x y))
```

### Implications from this example

Some scattered thoughts on what we can learn from this rule:
1. The right hand side is not always ISA-level, we'll need semantics that span our own definitions for CLIF/intermediate terms and that leverage existing ISA semantics.
2. Most rules use a mix of internal and external constructors. At first glance, many of the external constructors should be relatively simple to manually annotate as preconditions on the SMT terms.
3. We need a high-level model of registers, though we can probably sidestep any sort of nuanced memory models. 

## Another motivating example

A recent [developer-found ISLE Cranelift bug](https://bytecodealliance.zulipchat.com/#narrow/stream/217117-cranelift/topic/isle.20performance.20with.20cg_clif) in the x64 ISLE Cranelift backend might make an interesting goal to try to reproduce in an MVP version of a verification tool. 