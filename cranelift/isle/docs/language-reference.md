# ISLE: Instruction Selection Lowering Expressions

This document will describe ISLE (Instruction Selection Lowering
Expressions), a DSL (domain-specific language) that we have developed
in order to help us express certain parts of the Cranelift compiler
backend more naturally. ISLE was first [described in RFC
#15](https://github.com/bytecodealliance/rfcs/pull/15) and now is used
by and lives in the Cranelift tree in
[cranelift/isle](https://github.com/bytecodealliance/wasmtime/tree/main/cranelift/isle).

Documentation on how ISLE is used in Cranelift can be found
[here](../../docs/isle-integration.md).

## Intro and Whirlwind Tour: DSL for Instruction Lowering

The goal of ISLE is to represent *instruction lowering patterns*. An
instruction lowering pattern is a specification that a certain
combination of operators in the IR (CLIF), when combined under certain
conditions, can be compiled down into a certain sequence of machine
instructions. For example:

- An `iadd` (integer add) operator can always be lowered to an x86
  `ADD` instruction with two register sources.

- An `iadd` operator with one `iconst` (integer-constant) argument can
  be lowered to an x86 `ADD` instruction with a register and an
  immediate.

One could write something like the following in ISLE (simplified from
the real code [here](https://github.com/bytecodealliance/wasmtime/blob/main/cranelift/codegen/src/isa/x64/lower.isle)):

```lisp
;; Add two registers.
(rule (lower (iadd x y))
      (value_reg (add
                   (put_in_reg x)
                   (RegMemImm.Reg (put_in_reg y)))))

;; Add a register and an immediate.
(rule (lower (iadd x (simm32_from_value y))
      (value_reg (add
                   (put_in_reg x)
                   ;; `y` is a `RegMemImm.Imm`.
                   y)))
```

ISLE lets the compiler backend developer express this information in a
declarative way -- i.e., just write down a list of patterns, without
worrying how the compilation process tries them out -- and the ISLE
DSL compiler will convert this list of patterns into efficient Rust
code that becomes part of Cranelift.

The rest of this document will describe the semantics of the DSL
itself. ISLE has been designed to be a general-purpose DSL that can
apply to any sort of backtracking pattern-matching problem, and will
generate a decision tree in Rust that can call into arbitrary
interface code.

Separate documentation will describe how we have written *bindings*
and *helpers* to allow ISLE to be specifically used to write Cranelift
lowering patterns like the above. (TODO: link this documentation)

## Outline of This Document

This document is organized into the following sections:

* Term-Rewriting Systems: a general overview of how term-rewriting
  systems work, how to think about nested terms, patterns and rewrite
  rules, how they provide a general mechanism for computation, and how
  term-rewriting is often used in a compiler-implementation context.

* Core ISLE: the foundational concepts of the ISLE DSL, building upon
  a general-purpose term-rewriting base. Covers the type system (typed
  terms) and how rules are written.

* ISLE with Rust: covers how ISLE provides an "FFI" (foreign function
  interface) of sorts to allow interaction with Rust code, and
  describes the scheme by which ISLE execution is mapped onto Rust
  (data structures and control flow).[^1]

* ISLE Internals: describes how the ISLE compiler works. Provides
  insight into how an unordered collection of rewrite rules are
  combined into executable Rust code that efficiently traverses the
  input and matches on it.

[^1]: One might call this the BRIDGE (Basic Rust Interface Designed
    for Good Efficiency) to the ISLE, but unfortunately we missed the
    chance to introduce that backronym when we wrote the initial
    implementation.

## Background: Term-Rewriting Systems

*Note: this section provides general background on term-rewriting
systems that is useful to better understand the context for ISLE and
how to develop systems using it. Readers already familiar with
term-rewriting systems, or wishing to skip to details on ISLE's
version of term rewriting, can skip to the [next
section](#core-isle-a-term-rewriting-system).*

A [term-rewriting
system](https://en.wikipedia.org/wiki/Rewriting#Term_rewriting_systems),
or TRS, is a system that works by representing data as *terms* and
then applying *rules* to "rewrite" the terms. This rewrite process
continues until some application-specific end-condition is met, for
example until no more rules are applicable or until the term reaches a
"lowered" state by some definition, at which point the resulting term
is the system's output.

Term-rewriting systems are a general kind of computing system, at the
same level as (e.g.) Turing machines or other abstract computing
machines. Term-rewriting is actually Turing-complete, or in other
words, can express any program, if no limits are placed on term length
or recursion.[^2]

[^2]: In fact, the [lambda
      calculus](https://en.wikipedia.org/wiki/Lambda_calculus)
      introduced by Alonzo Church is actually a term-rewriting system
      and was developed at the same time as Turing's concepts of
      universal computation!

Why might one want to use a TRS rather than some other, more
conventional, way of computing an answer? One reason is that they are
highly applicable to *pattern-matching* problems: for example,
translating data in one domain to data in another domain, where the
translation consists of a bunch of specific equivalences. This is part
of why term-rewriting is so interesting in the compiler domain:
compiler backends work to lower certain patterns in the program (e.g.:
a multiply-add combination) into instructions that the target machine
provides (e.g.: a dedicated multiply-add instruction).

Term rewriting as a process also naturally handles issues of
*priority*, i.e. applying a more specific rule before a less specific
one. This is because the abstraction allows for multiple rules to be
"applicable", and so there is a natural place to reason about priority
when we choose which rule to apply. This permits a nice separation of
concerns: we can specify which rewrites are *possible* to apply
separately from which are *desirable* to apply, and adjust or tune the
latter (the "strategy") at will without breaking the system's
correctness.

Additionally, term rewriting allows for a sort of *modularity* that is
not present in hand-written pattern-matching code: the specific rules
can be specified in any order, and the term-rewriting engine "weaves"
them together so that in any state, when we have partially matched the
input and are narrowing down which rule will apply, we consider all
the related rules at once. Said another way: hand-written code tends
to accumulate a lot of nested conditionals and switch/match
statements, i.e., resembles a very large decision tree, while
term-rewriting code tends to resemble a flat list of simple patterns
that, when composed and combined, become that more complex tree. This
allows the programmer to more easily maintain and update the set of
lowering rules, considering each in isolation.

### Data: Nested Trees of Constructors

A term-rewriting system typically operates on data that is in a *tree*
form, or at least can be interpreted that way.[^3]

[^3]: In the most fundamental and mathematical sense, a TRS just
      operates on a sequence of symbols, but we can talk about
      structure that is present in those symbols in any well-formed
      sequence. For example, we can define a TRS that only operates on
      terms with balanced parentheses; then we have our tree.

In ISLE and hence in this document, we operate on terms that are
written in an
[S-expression](https://en.wikipedia.org/wiki/S-expression) syntax,
borrowed from the Lisp world. So we might have a term:

```lisp
    (a (b c 1 2) (d) (e 3 4))
```

which we can write more clearly as the tree:

```lisp
    (a
      (b
        c 1 2)
      (d)
      (e
        3 4))
```

Each term consists of either a *constructor* (which looks like a
function call to Lisp-trained eyes) or a *primitive*. In the above,
the `(a ...)`, `(b ...)`, `(d)`, and `(e ...)` terms/subterms are
constructor invocations. A constructor takes some number of arguments
(its *arity*), each of which is itself a term. Primitives can be
things like integer, string, or boolean constants, or variable names.

Some term-rewriting systems have other syntax conventions: for
example, systems based on
[Prolog](https://en.wikipedia.org/wiki/Prolog) tend to write terms
like `a(b(c, 1, 2), d, e(3, 4))`, i.e., with the name of the term on
the outside of the parentheses. This is just a cosmetic difference to
the above, but we note it to make clear that the term structure is
important, not the syntax.

It may not be immediately clear how to use this data representation,
but we can give a small flavor here: if one defines *constructors* for
each instruction or operator in a compiler's intermediate
representation (IR), one can start to write expressions from that IR
as terms; for example:

```lisp
    v1 = imul y, z
    v2 = iadd x, v1
```

could become:

```lisp
    (iadd x (imul y z))
```

This will become much more useful once we have rewrite rules to
perform transformations on the terms!

Representing an IR is, of course, just one possible use of term data
(albeit the original "MVP" that guided ISLE's design); there are many
others, too. Interested readers are encouraged to read more on, e.g.,
[Prolog](https://en.wikipedia.org/wiki/Prolog), which has been used to
represent logical predicates, "facts" in expert systems, symbolic
mathematical terms, and more.

### Rules: Left-hand-side Patterns, Right-hand-side Expressions

The heart of a term-rewriting system is in the set of *rules* that
actually perform the rewrites. The "program" itself, in a
term-rewriting DSL, consists simply of an unordered list of
rules. Each rule may or may not apply; if it applies, then it can be
used to edit the term. Execution consists of repeated application of
rules until some criteria are met.

A rule consists of two parts: the left-hand side (LHS), or *pattern*,
and right-hand side (RHS), or *expression*. The left-hand and
right-hand nomenclature comes from a common way of writing rules as:

```plain
    A -> B              ;; any term "A" is rewritten to "B"

    (A x) -> (B (C x))  ;; any term (A x), for some x, is rewritten to (B (C x)).

    (A _) -> (D)        ;; any term (A _), where `_` is a wildcard (any subterm),
                        ;; is rewritten to (D).
```

#### Left-hand Sides: Patterns

Each left-hand side is written in a pattern language that commonly has
a few different kinds of "matchers", or operators that can match
subterms:

* `(A pat1 pat2 ...)` matches a constructor `A` with patterms for each
  of its arguments.

* `x` matches any subterm and captures its value in a variable
  binding, which can be used later when we specify the right-hand side
  (so that the rewrite contains parts of the original term).

* `_` is a wildcard and matches anything, without capturing it.

* Primitive constant values, such as `42` or `$Symbol`, match only if
  the term is exactly equal to this constant.

These pattern-matching operators can be combined, so we could write,
for example, `(A (B x _) z)`. This pattern would match the term `(A (B
1 2) 3)` but not `(A (C 4 5) 6)`.

A pattern can properly be seen as a partial function from input term
to captured (bound) variables: it either matches or it doesn't, and if
it does, it provides specific term values for each variable binding
that can be used by the right-hand side.

A fully-featured term rewriting system usually has other operators as
well, for convenience: for example, "match already-captured value", or
"bind variable to subterm and also match it with subpattern", or
"match subterm with all of these subpatterns". But even the above is
powerful enough for Turing-complete term reduction, surprisingly; the
essence of term-rewriting's power is just its ability to trigger
different rules on different "shapes" of the tree of constructors in
the input, and on special cases for certain argument values.

Pattern-based term rewriting has a notable and important feature: it
typically allows *overlapping* rules. This means that more than one
pattern might match on the input. For example, the two rules:

```plain
    (A (B x)) -> (C x)
    (A _) -> (D)
```

could *both* apply to an input term `(A (B 1))`. The first rule would
rewrite this input to `(C 1)`, and the second rule would rewrite it to
`(D)`. Either rewrite would be an acceptable execution step under the
base semantics of most term-rewriting systems; ordinarily, the
*correctness* of the rewrite should not depend on which rule is
chosen, only possibly the "optimality" of the output (whatever that
means for the application domain in question) or the number of rewrite
steps to get there.

However, in order to provide a deterministic answer, the system must
somehow specify which rule will be applied in such a situation based
on precedence, or specificity, or some other tie-breaker. A common
heuristic is "more specific rule wins". We will see how ISLE resolves
this question below by using both this heuristic and an explicit
priority mechanism.[^4]

[^4]: Some term-rewriting systems actually elaborate the entire space
      of possibilities, following *all* possible rule application
      sequences / rewrite paths. For example, the *equality
      saturation* technique
      ([paper](https://cseweb.ucsd.edu/~lerner/papers/popl09.pdf),
      [example implementation
      Egg](https://blog.sigplan.org/2021/04/06/equality-saturation-with-egg/))
      builds a data structure that represents all equivalent terms
      under a set of rewrite rules, from which a heuristic
      (cost/goodness function) can be used to extract one answer when
      needed.

#### Right-hand Sides: Rewrite Expressions

Given a rule whose pattern has matched, we now need to compute the
rewritten term that replaces the original input term. This rewrite is
specified by the right-hand side (RHS), which consists of an
*expression* that generates a new term. This expression can use parts
of the input term that have been captured by variables in the
pattern.

We have already seen a few examples of this above: simple term
expressions, with variables used in place of concrete subterms where
desired. A typical term-rewrite system allows just a few options in
the output expression:

* Terms, with sub-expressions as arguments;
* Constant primitives (`42` or `$Symbol`); and
* Captured variable values (`x`).

The options are more limited in expressions than in patterns (e.g.,
there are no wildcards) because a pattern is matching on a range of
possible terms while an expression must specify a specific rewrite
result.

### Rewrite Steps and Intermediate Terms

Now that we can specify rewrites via a list of rules, we can study how
the top-level execution of a term-rewriting system proceeds. Much of
the power of term-rewriting comes from the fact that rewrites can
*chain together* into a multi-step traversal through several
intermediate terms before the final answer is computed.

For a simple example, consider the following rules:

```plain
    (A (B x)) -> (C x)
    (C (D x)) -> (E x)
    (C (F x)) -> (G x)
```

This set of rules will rewrite `(A (B (D 42)))` to `(C (D 42))`, then
to `(E 42)` (via the first and second rules respectively).

How is this useful? First, rewriting one term to another (here, `C` at
the top level) that in turn appears in the left-hand side of other
rules allows us to *factor* a "program" of term-rewriting rules in the
same way that imperative programs are factored into separate
functions.[^5] The usual advantages of a well-factored program, where
each problem is solved with a small step that "reduces to a previously
solved problem", apply here.

Second, repeating the rewrite step is actually what grants
term-rewriting its Turing-completeness: it allows for arbitrary
control flow.[^6] This might be useful in cases where, for example, a
term-rewriting program needs "loop" over a list of elements in the
input: it can recurse and use intermediate terms to store state.

While this full generality may not be used often in the
domain-specific applications of term-rewriting that emphasize its
pattern-matching (such as instruction selectors), the user should not
be afraid to define and use intermediate terms -- rewriting into them,
then defining additional rules to rewrite further -- when it helps to
factor common behavior out of multiple rules, or aids in conceptual
clarity.

[^5]: In fact, ISLE actually compiles rules for different top-level
      pattern terms (`(A ...)` and `(C ...)` in the example) into
      separate Rust functions, so factoring rules to use intermediate
      terms can provide code-size and compile-time benefits for the
      ISLE-generated Rust code as well.

[^6]: The [lambda calculus' reduction
      rules](https://en.wikipedia.org/wiki/Lambda_calculus#Reduction)
      are a good example of this.

### Application to Compilers: A Term is a Value; Rewrites are Lowerings

So far this has been a fairly abstract introduction to term-rewriting
systems as a general computing paradigm. How does this relate (or, how
is it commonly mapped) to the instruction-selection problem?

In a domain such as instruction selection, we manipulate terms that
represent computations described by an IR, and the terms are
eventually rewritten into terms that name specific machine
instructions. We can think of each term as having a denotational value
that that *is* that program value. Then, any rewrite is correct if it
preserves the denotational value of the term.

In other words, terms are just values, and rules specify alternative
ways to compute the same values. We might have rewrite rules that
correspond to common algebraic identities (`a + b` == `b + a`, and
`a + 0` == `a`), for example. The main sort of rewrite rule, however,
will be one that takes a machine-*independent* operator term and
rewrites it into a machine-*dependent* instruction term. For example:

```plain
    (iadd a b) -> (isa.add_reg_reg a b)

    (iadd a (iconst 0)) -> a

    (iadd a (iconst n)) (isa.add_reg_imm a n)
```

These rules specify three ways to convert an IR `iadd` operator into
an ISA-specific instruction. Recall from above that in general, an
application of a term-rewriting system should not depend for
correctness on the order or choice of rule application: when multiple
rules are applicable, then any sequence of rewrites that ends in a
terminating state (a state with no further-applicable rules) should be
considered a "correct" answer.[^7] Here, this is true: if, for
example, we choose the register-plus-register form (the first rule)
for an `iadd` operation, but the second argument is actually an
`iconst`, then that is still valid, and the `iconst` will separately
be rewritten by some other rule that generates a constant into a
register. It simply may not be as efficient as the more specific third
rule (or second rule, if the constant is zero). Hence, rule ordering
and prioritization is nevertheless important for the quality of the
instruction selector.

[^7]: Note that this suggests an interesting testing strategy: we
      could choose arbitrary (random) orders of lowering rules to
      apply, or even deliberately worst-case orders according to some
      heuristic. If we can differentially test programs compiled with
      such randomized lowerings against "normally" compiled programs
      and show that the results are always the same, then we have
      shown that are lowering rules are "internally consistent",
      without any other external oracle. This will have a similar
      effect to
      [wasm-mutate](https://github.com/bytecodealliance/wasm-tools/tree/main/crates/wasm-mutate),
      but takes mutations implicitly from the pool of rules rather
      than a separate externally-defined pool of mutations. This idea
      remains future work.

## Core ISLE: a Term-Rewriting System

This section describes the core ISLE language. ISLE's core is a
term-rewriting system, with a design that very closely follows the
generic concepts that we have described above.

In the core language, ISLE's key departure from many other
term-rewriting systems is that it is *strongly typed*. A classical
term-rewriting system, especially one designed for instruction
rewriting, will typically have just one type of term, corresponding to
a "value" in the program. In contrast, ISLE is designed so that terms
can represent various concepts in a compiler backend: values, but also
machine instructions or parts of those instructions ("integer
immediate value encodable in machine's particular format"), or
abstract bundles of information with invariants or guarantees encoded
in the type system ("load that I can sink", "instruction that produces
flags").

ISLE's other key departure from many other systems is its first-class
integration with Rust, including a well-defined "FFI" mapping that
allows ISLE rules to call into Rust in both their patterns and
expressions, and to operate directly on types that are defined in the
surrounding Rust code. This allows for easy and direct embedding into
an existing compiler backend. We will cover this aspect more in the
next section, [ISLE to Rust](#isle-to-rust).

### Rules

ISLE allows the user to specify rewrite rules, with a syntax similar
in spirit to that shown above:

```lisp
    (rule
      ;; left-hand side (pattern): if the input matches this ...
      (A (B _ x) (C y))
      ;; ... then rewrite to this:
      (D x y))
```

The pattern (left-hand side) is made up of the following match
operators:

* Wildcards (`_`).
* Integer constants (decimal/hex/binary/octal, positive/negative: `1`, `-1`,
  `0x80`, `-0x80`). Hex constants can start with either `0x` or `0X`.
  Binary constants start with `0b`. Octal constants start with `0o`.
  Integers can also be interspersed with `_` as a separator, for example
  `1_000` or `0x1234_5678`, for readability.
* constants imported from the embedding, of arbitrary type
  (`$MyConst`).
* Variable captures and matches (bare identifiers like `x`; an
  identifier consists of alphanumeric characters and underscores, and
  does not start with a digit). The first occurrence of a variable `x`
  captures the value; each subsequent occurrence matches on the
  already-captured value, rejecting the match if not equal.
* Variable captures with sub-patterns: `x @ PAT`, which captures the
  subterm in `x` as above but also matches `PAT` against the
  subterm. For example, `x @ (A y z)` matches an `A` term and captures
  its arguments as `y` and `z`, but also captures the whole term as
  `x`.
* conjunctions of subpatterns: `(and PAT1 PAT2 ...)` matches all of
  the subpatterns against the term. If any subpattern does not match,
  then this matcher fails.
* Term deconstruction: `(term PAT1 PAT2 ...)`, where `term` is a
  defined term (type variant or constructor) and the subpatterns are
  applied to each argument value in turn. Note that `term` cannot be a
  wildcard; it must be a specific, concrete term.

The expression (right-hand side) is made up of the following
expression operators:

* Integer and symbolic constants, as above.
* Variable uses (bare `x` identifier).
* Term constructors (`(term EXPR1 EXPR2 ...)`, where each
  subexpression is evaluated first and then the term is constructed).
* `let`-expressions that bind new variables, possibly using the values
  multiple times within the body: `(let ((var1 type1 EXPR1) (var2 ...)
  ...) BODY ...)`. Each variable's initialization expression can refer
  to the immediately previous variable bindings (i.e., this is like a
  `let*` in Scheme). `let`s are lexically-scoped, meaning that bound
  variables are available only within the body of the `let`.

When multiple rules are applicable to rewrite a particular term, ISLE
will choose the "more specific" rule according to a particular
heuristic: in the lowered sequence of matching steps, when one
left-hand side completes a match while another with the same prefix
continues with further steps, the latter (more specific) is chosen.

The more-specific-first heuristic is usually good enough, but when an
undesirable choice occurs, explicit priorities can be specified.
Rules with explicit priorities are written as `(rule PRIO lhs rhs)`
where `PRIO` is a signed (positive or negative) integer. An applicable
rule with a higher priority will always match before a rule with a
lower priority. The default priority for all rules if not otherwise
specified is `0`.

Note that the system allows multiple applicable rules to exist with
the same priority: that is, while the priority system allows for
manual tie-breaking, this tie-breaking is not required.

Finally, one important note: the priority system is considered part of
the core language semantics and execution of rules with different
priorities is well-defined, so can be relied upon when specifying
correct rules. However, the tie-breaking heuristic is *not* part of
the specified language semantics, and so the user should never write
rules whose correctness depends on one rule overriding another
according to the heuristic.

### Typed Terms

ISLE allows the programmer to define types, and requires every term to
have *parameter types* and a *return type* (analogous to first-order
functions).

The universe of types is very simple: there are *primitives*, which
can be integers or symbolic constants (imported from the Rust
embedding), and *enums*, which correspond directly to Rust enums with
variants that have named fields. There is no subtyping. Some examples
of type definitions are:

```lisp

    (type u32 (primitive u32))  ;; u32 is a primitive, and is
                                ;; spelled `u32` in the generated Rust code.

    (type MyType (enum
                   (A (x u32) (y u32))
                   (B (z u32))
                   (C)))        ;; MyType is an enum, with variants
                                ;; `MyType::A { x, y }`, `MyType::B { z }`,
                                ;; and `MyType::C`.

    (type MyType2 extern (enum (A)))
                                ;; MyType2 is an enum with variant `MyType2::A`.
                                ;; Its type definition is not included in the
                                ;; generated Rust, but rather, assumed to exist
                                ;; in surrounding code. Useful for binding to
                                ;; existing definitions.
```

We then declare constructors with their parameter and return types as
follows:

```lisp

    (decl Term1 (u32 u32) MyType)  ;; Term1 has two `u32`-typed parameters,
                                   ;; and itself has type `MyType`.
    (decl Term2 () u32)            ;; Term2 has no parameters and type `u32`.
```

Note that when an enum type is defined, its variants are implicitly
defined as constructors as well. These constructors are namespaced
under the name of the type, to avoid ambiguity (or the need to do
type-dependent lookups in the compiler, which can complicate type
inference). For example, given the above `MyType` definitions, we
automatically have the following constructors:

```lisp

    ;; These definitions are implicit and do not need to be written (doing
    ;; so is a compile-time error, actually). We write them here just to
    ;; show what they would look like.

    (decl MyType.A (u32 u32) MyType)
    (decl MyType.B (u32) MyType)
    (decl MyType.C () MyType)

    (decl MyType2.A () MyType2)
```

### Why Types?

For terms that are not enum variants, the notion that a term "has a
type" is somewhat foreign to a classical term-rewriting system. In
most formal symbolic systems, the terms are manipulated as opaque
sequences or trees of symbols; they have no inherent meaning other
than what the user implicitly defines with the given rules. What does
it mean for a term to "have a type" when it is just data? Or, said
another way: why isn't the type of `Term2` just `Term2`?

The types come into play when we define *rules*: one term of type `T`
can only be rewritten into another term of type `T`, and when a
parameter has a certain type, only subterms with that type can
appear. Without explicit types on terms and their parameters, any term
could be rewritten to any other, or substituted in as a parameter, and
there is thus a kind of dynamic typing about which the programmer must
have some caution. In most applications of a term-rewriting system,
there is already some de-facto "schema": some parameter of a term
representing a machine instruction can only take on one of a few
subterms (representing, say, different addressing modes). ISLE's types
just make this explicit.

Thus, the first answer to "why types" is that they enforce a schema on
the terms, allowing the programmer to have stronger well-formed-data
invariants.

The second reason is that the types are an integral part of the
compilation-to-Rust strategy: every constructor actually does evaluate
to a Rust value of the given "return value" type, given actual Rust
values for its parameters of the appropriate parameter types. We will
see more on this below.

### Well-Typed Rules and Type Inference

Now that we understand how to define types, let's examine in more
detail how they are used to verify that the pattern and rewrite
expression of a rule have the same type.

ISLE uses a simple unidirectional type-inference algorithm that
propagates type information through the pattern, resulting in a "type
environment" that specifies the type for each captured variable, and
then uses this to typecheck the rewrite expression. The result of this
is that types are almost completely inferred, and are only annotated
in a few places (`let` bindings specifically).

The typing rules for patterns in ISLE are:

* At the root of the pattern, we require that a *constructor* pattern
  is used, rather than some other match operation (a wildcard, integer
  constant, etc.). This is because compilation and dispatch into rules
  is organized by the top-level constructor of the term being
  rewritten.

* At each part of the pattern except the root, there is an "expected
  type" that is inferred from the surrounding context. We check that
  this matches the actual type of the pattern.

* A constructor pattern `(C x y z)`, given a constructor `(decl C (T1
  T2 T2) R)`, has type `R` and provides expected types `T1`, `T2`, and
  `T3` to its subpatterns.

* A variable capture pattern `x` is compatible with any expected type
  the first time it appears, and captures this expected type under the
  variable identifier `x` in the type environment. Subsequent
  appearances of `x` check that the expected type matches the
  already-captured type.

* A conjunction `(and PAT1 PAT2 ...)` checks that each subpattern is
  compatible with the expected type.

* Integer constants are compatible with any primitive expected
  type. (This may change in the future if we add non-numeric
  primitives, such as strings.)

If we are able to typecheck the pattern, we have a type environment
that is a map from variable bindings to types: e.g., `{ x: MyType, y:
MyType2, z: u32 }`. We then typecheck the rewrite expression.

* Every expression also has an expected type, from the surrounding
  context. We check that the provided expression matches this type.

* The top-level rewrite expression must have the same type as the
  top-level constructor in the pattern. (In other words, a term can
  only be rewritten to another term of the same type.)

* Constructors check their return values against the expected type,
  and typecheck their argument expressions against their parameter
  types.

* A `let` expression provides types for additional variable bindings;
  these are added to the type environment while typechecking the
  body. The expected type for the body is the same as the expected
  type for the `let` itself.

### A Note on Heterogeneous Types

We should illuminate one particular aspect of the ISLE type system
that we described above. We have said that a term must be rewritten to
another term of the same type. Note that this does *not* mean that,
for example, a set of ISLE rules cannot be used to translate a term of
type `T1` to a term of type `T2`. The trick is to define a top-level
"driver" that wraps the `T1`, such that reducing this term results in
a `T2`. Concretely:

```lisp
    (type T1 ...)
    (type T2 ...)

    (decl Translate (T1) T2)

    (rule (Translate (T1.A ...))
          (T2.X ...))
    (rule (Translate (T1.B ...))
          (T2.Y ...))
```

This gets to the heart of rewrite-system-based computation, and has
relevance for applications of ISLE to compiler backends. A common
technique in rewrite systems is to "kick off" a computation by
wrapping a term in some intermediate term that then drives a series of
reductions. Here we are using `Translate` as this top-level term. A
difference between ISLE and some other rewrite-based instruction
selectors is that rewrites are always driven by term reduction from
such a toplevel term, rather than a series of equivalences directly
from IR instruction to machine instruction forms.

In other words, a conventional instruction selection pattern engine
might let one specify `(Inst.A ...) -> (Inst.X ...)`. In this
conventional design, the instruction/opcode type on the LHS and RHS
must be the same single instruction type (otherwise rewrites could not
be chained), and rewrite relation (which we wrote as `->`) is in
essence a single privileged relation. One can see ISLE as a
generalization: we can define many different types, and many different
toplevel terms from which we can start the reduction. In principle,
one could have:

```lisp

    (type IR ...)
    (type Machine1 ...)
    (type Machine2 ...)

    (decl TranslateToMachine1 (IR) Machine1)
    (decl TranslateToMachine2 (IR) Machine2)

    (rule (TranslateToMachine1 (IR.add a b)) (Machine1.add a b))
    (rule (TranslateToMachine2 (IR.add a b)) (Machine2.weird_inst a b))
```

and then both translations are available. We are "rewriting" from `IR`
to `Machine1` and from `IR` to `Machine2`, even if rewrites always
preserve the same type; we get around the rule by using a constructor.

### Constructors and Extractors

So far, we have spoken of terms and constructors: a term is a schema
for data, like `(A arg1 arg2)`, while we have used the term
"constructor" to refer to the `A`, like a function. We now refine this
notion somewhat and define what it means for a term to appear in the
left-hand (pattern) or right-hand (expression) side of a rule.

More precisely, a term, like `A`, can have three kinds of behavior
associated with it: it can be an enum type variant, it can be a
constructor, or it can be an *extractor*, which we will define in a
moment. A term can be both an extractor and constructor
simultaneously, but the enum type variant case is mutually exclusive
with the others.

The distinction between a "constructor" and an "extractor" is whether
a term is being deconstructed (matched on) -- by an extractor -- or
constructed -- by a constructor.

#### Constructors

Constructor behavior on a term allows it to be invoked in the
right-hand side of a rule. A term can have either an "external
constructor" (see below) or an "internal constructor", defined in
ISLE. Any term `A` that has one or more `(rule (A ...) RHS)` rules in
the ISLE source implicitly has an internal constructor, and this
constructor can be invoked from the right-hand side of other rules.

#### Extractors

Extractor behavior on a term allows it to be used in a pattern in the
left-hand side of a rule. If one considers a constructor to be a
function that goes from argument values to the complete term, then an
extractor is a function that takes a complete term and possibly
matches on it (it is fallible). If it does match, it provides the
arguments *as results*.

One can see extractors as "programmable match operators". They are a
generalization of enum-variant deconstruction. Where a traditional
term-rewriting system operates on a term data-structure that exists in
memory, and can discover that a pattern `(A x y)` matches a term `A`
at a particular point in the input, an extractor-based system instead
sees `A` as an *arbitrary programmable operator* that is invoked
wherever a pattern-match is attempted, and can return "success" with
the resulting "fields" as if it were actually an enum variant. For
more on this topic, see the motivation and description in [RFC 15
under "programmable matching on virtual
nodes"](https://github.com/bytecodealliance/rfcs/blob/main/accepted/cranelift-isel-isle-peepmatic.md#extractors-programmable-matching-on-virtual-nodes).

To provide a concrete example, if we have the term declarations

```lisp
    (decl A (u32 u32) T)
    (decl B (T) U)
```

then if we write a rule like

```lisp
    (rule (B (A x y))
          (U.Variant1 x y))
```

then we have used `A` as an *extractor*. When `B` is invoked as a
constructor with some `T`, this rule uses `A` as an extractor and
attempts (via whatever programmable matching behavior) to use `A` to
turn the `T` into two `u32`s, binding `x` and `y`. `A` can succeed or
fail, just as any other part of a pattern-match can.

Just as for constructors, there are *internal* and *external*
extractors. Most of the interesting programmable behavior occurs in
external extractors, which are defined in Rust; we will discuss this
further in a section below. Internal extractors, in contrast, behave
like macros, and can be defined for convenience: for example, we can
write

```lisp
    (decl A (u32 u32) T)
    (extractor (A pat1 pat2)
               (and
                 (extractArg1 pat1)
                 (extractArg2 pat2)))
```

which will, for example, expand a pattern `(A (subterm ...) _)` into
`(and (extractArg1 (subterm ...)) (extractArg2 _))`: in other words,
the arguments to `A` are substituted into the extractor body and then
this body is inlined.

#### Implicit Type Conversions

For convenience, ISLE allows the program to associate terms with pairs
of types, so that type mismatches are *automatically resolved* by
inserting that term.

For example, if one is writing a rule such as

```lisp
    (decl u_to_v (U) V)
    (rule ...)

    (decl MyTerm (T) V)
    (rule (MyTerm t)
          (u_to_v t))
```

the `(u_to_v t)` term would not typecheck given the ISLE language
functionality that we have seen so far, because it expects a `U` for
its argument but `t` has type `T`. However, if we define


```lisp
    (convert T U t_to_u)

    ;; For the above to be valid, `t_to_u` should be declared with the
    ;; signature:
    (decl t_to_u (T) U)
    (rule ...)
```

then the DSL compiler will implicitly understand the above `MyTerm` rule as:

```lisp
    (rule (MyTerm t)
          (u_to_v (t_to_u t)))
```

This also works in the extractor position: for example, if one writes

```lisp
    (decl defining_instruction (Inst) Value)
    (extern extractor defining_instruction ...)

    (decl iadd (Value Value) Inst)

    (rule (lower (iadd (iadd a b) c))
          ...)

    (convert Inst Value defining_instruction)
```

then the `(iadd (iadd a b) c)` form will be implicitly handled like
`(iadd (defining_instruction (iadd a b)) c)`. Note that the conversion
insertion needs to have local type context in order to find the right
converter: so, for example, it cannot infer a target type from a
pattern where just a variable binding occurs, even if the variable is
used in some typed context on the right-hand side. Instead, the
"inner" and "outer" types have to come from explicitly typed terms.

#### Summary: Terms, Constructors, and Extractors

We start with a `term`, which is just a schema for data:

```lisp
    (decl Term (A B C u32 u32) T)
```

A term can have:

1. A single internal extractor body, via a toplevel `(extractor ...)`
   form, OR

2. A single external extractor binding (see next section); AND

3. One or more `(rule (Term ...) ...)` toplevel forms, which together
   make up an internal constructor definition, OR

4. A single external constructor binding (see next section).

### If-Let Clauses

As an extension to the basic left-hand-side / right-hand-side rule
idiom, ISLE allows *if-let clauses* to be used. These add additional
pattern-matching steps, and can be used to perform additional tests
and also to use constructors in place of extractors during the match
phase when this is more convenient.

To introduce the concept, an example follows (this is taken from the
[RFC](https://github.com/bytecodealliance/rfcs/tree/main/isle-extended-patterns.md)
that proposed if-lets):

```lisp
;; `u32_fallible_add` can now be used in patterns in `if-let` clauses
(decl pure u32_fallible_add (u32 u32) u32)
(extern constructor u32_fallible_add u32_fallible_add)

(rule (lower (load (iadd addr
                         (iadd (uextend (iconst k1))
                               (uextend (iconst k2))))))
      (if-let k (u32_fallible_add k1 k2))
      (isa_load (amode_reg_offset addr k)))
```

The key idea is that we allow a `rule` form to contain the following
sub-forms:

```lisp
(rule LHS_PATTERN
  (if-let PAT2 EXPR2)
  (if-let PAT3 EXPR3)
  ...
  RHS)
```

The matching proceeds as follows: the main pattern (`LHS_PATTERN`)
matches against the input value (the term to be rewritten), as
described in detail above. Then, if this matches, execution proceeds
to the if-let clauses in the order they are specified. For each, we
evaluate the expression (`EXPR2` or `EXPR3` above) first. An
expression in an if-let context is allowed to be "fallible": the
constructors return `Option<T>` at the Rust level and can return
`None`, in which case the whole rule application fails and we move on
to the next rule as if the main pattern had failed to match. (More on
the fallible constructors below.) If the expression evaluation
succeeds, we match the associated pattern (`PAT2` or `PAT3` above)
against the resulting value. This too can fail, causing the whole rule
to fail. If it succeeds, any resulting variable bindings are
available. Variables bound in the main pattern are available for all
if-let expressions and patterns, and variables bound by a given if-let
clause are available for all subsequent clauses. All bound variables
(from the main pattern and if-let clauses) are available in the
right-hand side expression.

#### Pure Expressions and Constructors

In order for an expression to be used in an if-let clause, it has to
be *pure*: it cannot have side-effects. A pure expression is one that
uses constants and pure constructors only. Enum variant constructors
are always pure. In general constructors that invoke function calls,
however (either as internal or external constructor calls), can lead
to arbitrary Rust code and have side-effects. So, we add a new
annotation to declarations as follows:

```lisp
;; `u32_fallible_add` can now be used in patterns in `if-let` clauses
(decl pure u32_fallible_add (u32 u32) u32)

;; This adds a method
;; `fn u32_fallible_add(&mut self, _: u32, _: u32) -> Option<u32>`
;; to the `Context` trait.
(extern constructor u32_fallible_add u32_fallible_add)
```

The `pure` keyword here is a declaration that the term, when used as a
constructor, has no side-effects. Declaring an external constructor on
a pure term is a promise by the ISLE programmer that the external Rust
function we are naming (here `u32_fallible_add`) has no side-effects
and is thus safe to invoke during the match phase of a rule, when we
have not committed to a given rule yet.

When an internal constructor body is generated for a term that is pure
(i.e., if we had `(rule (u32_fallible_add x y) ...)` in our program
after the above declaration instead of the `extern`), the right-hand
side expression of each rule that rewrites the term is also checked
for purity.

#### `partial` Expressions

ISLE's `partial` keyword on a term indicates that the term's
constructors may fail to match, otherwise, the ISLE compiler assumes
the term's constructors are infallible.

For example, the following term's constructor only matches if the value
is zero:

```
;; Match any zero value.
(decl pure partial is_zero_value (Value) Value)
(extern constructor is_zero_value is_zero_value)
```

Internal constructors without the `partial` keyword can
only use other constructors that also do not have the `partial` keyword.

#### `if` Shorthand

It is a fairly common idiom that if-let clauses are used as predicates
on rules, such that their only purpose is to allow a rule to match,
and not to perform any destructuring with a sub-pattern. For example,
one might want to write:

```lisp
(rule (lower (special_inst ...))
      (if-let _ (isa_extension_enabled))
      (isa_special_inst ...))
```

where `isa_extension_enabled` is a pure constructor that is fallible,
and succeeds only when a condition is true.

To enable more succinct expression of this idiom, we allow the
following shorthand notation using `if` instead:

```lisp
(rule (lower (special_inst ...))
      (if (isa_extension_enabled))
      (isa_special_inst ...))
```

## ISLE to Rust

Now that we have described the core ISLE language, we will document
how it interacts with Rust code. We consider these interactions to be
semantically as important as the core language: they are not
implementation details, but rather, a well-defined interface by which
ISLE can interface with the outside world (an "FFI" of sorts).

### Mapping to Rust: Constructors, Functions, and Control Flow

ISLE was designed to have a simple, easy-to-understand mapping from
its language semantics to Rust semantics. This means that the
execution of ISLE rewriting has a well-defined implementation in
Rust. The basic principles are:

1. Every term with rules in ISLE becomes a single Rust function. The
   arguments are the Rust function arguments. The term's "return
   value" is the Rust function's return value (wrapped in an `Option`
   because pattern coverage can be incomplete).

2. One rewrite step is one Rust function call.

3. Rewriting is thus eager, and reified through ordinary Rust control
   flow. When we construct a term that appears on the left-hand side
   of rules, we do so by calling a function (the "constructor body");
   and this function *is* the rewrite logic, so the term is rewritten
   as soon as it exists. The code that embeds the ISLE generated code
   will kick off execution by calling a top-level "driver"
   constructor. The body of the constructor will eventually choose one
   of several rules to apply, and execute code to build the right-hand
   side expression; this can invoke further constructors for its
   subparts, kicking off more rewrites, until eventually a value is
   returned.

4. This design means that "intermediate terms" -- constructed terms
   that are then further rewritten -- are never actually built as
   in-memory data-structures. Rather, they exist only as ephemeral
   stack-frames while the corresponding Rust function executes. This
   means that there is very little or no performance penalty to
   factoring code into many sub-rules (subject only to function-call
   overhead and/or the effectiveness of the Rust inliner).

5. Backtracking -- attempting to match rules, and backing up to follow
   a different path when a match fails -- exists, but is entirely
   internal to the generated Rust function for rewriting one
   term. Once we are rewriting a term, we have committed to that term
   existing as a rewrite step; we cannot backtrack further. However,
   backtracking can occur within the delimited scope of this one
   term's rewrite; we have a phase during which we evaluate left-hand
   sides, trying to find a matching rule, and once we find one, we
   commit and start to invoke constructors to build the right-hand
   side.

   Said another way, the principle is that left-hand sides can be
   fallible, and have no side-effects as they execute; right-hand
   sides, in contrast, are infallible. This simplifies the control
   flow and makes reasoning about side-effects (especially with
   respect to external Rust actions) easier.

This will become more clear as we look at how Rust interfaces are
defined, and how the generated code appears, below.

### Extern Constructors and Extractors

ISLE programs interact with the surrounding Rust code in which they
are embedded by allowing the programmer to define a term to have
*external constructors* and an *external extractor*.

The design philosophy of ISLE is that while internally it operates as
a fairly standard term-rewriting system, on the boundaries the "terms"
should be virtual, and defined procedurally rather than reified into
data structures, in order to allow for very flexible binding to the
embedding application. Thus, when term-rewriting bottoms out on the
input side, it just calls "extractors" to match on whatever ultimate
input the user provides, and these are fully programmable; and when it
bottoms out on the output side, the "term tree" is reified as a tree
of Rust function calls rather than plain data.

#### Constructors

As we defined above, a "constructor" is a term form that appears in an
expression and builds its return value from its argument
values. During the rewriting process, a constructor that can trigger
further rewriting rules results in a Rust function call to the body of
the "internal constructor" built from these rules; the term thus never
exists except as argument values on the stack. However, ultimately the
ISLE code needs to return some result to the outside world, and this
result may be built up of many parts; this is where *external
constructors* come into play.

For any term declared like

```lisp
    (decl T (A B C) U)
```

the programmer can declare

```lisp
    (extern constructor T ctor_func)
```

which means that there is a Rust function `ctor_func` on the context
trait (see below) that can be *invoked* with arguments of type `A`,
`B`, `C` (actually borrows `&A`, `&B`, `&C`, for non-primitive types)
and returns a `U`.

External constructors are infallible: that is, they must succeed, and
always return their return type. In contrast, internal constructors
can be fallible because they are implemented by a list of rules whose
patterns may not cover the entire domain (in which case, the term
should be marked `partial`). If fallible behavior is needed when
invoking external Rust code, that behavior should occur in an extractor
(see below) instead: only pattern left-hand sides are meant to be
fallible.

#### Extractors

An *external extractor* is an implementation of matching behavior in
left-hand sides (patterns) that is fully programmable to interface
with the embedding application. When the generated pattern-matching
code is attempting to match a rule, and has a value to match against
an extractor pattern defined as an external extractor, it simply calls
a Rust function with the value of the term to be deconstructed, and
receives an `Option<(arg1, arg2, ...)>` in return. In other words, the
external extractor can choose to match or not, and if it does, it
provides the values that are in turn matched by sub-patterns.

For any term declared like

```lisp
    (decl T (A B C) U)`
```

the programmer can declare

```lisp
    (extern extractor T etor_func)
```

which means that there is a Rust function `etor_func` on the context
trait (see below) that can be *invoked* with an argument of type `&U`,
and returns an `Option<(A, B, C)>`.

If an extractor returns `None`, then the generated matching code
proceeds just as if an enum variant match had failed: it moves on to
try the next rule in turn.

### Mapping Type Declarations to Rust

When we declare a type like

```lisp
    (decl MyEnum (enum
                   (A (x u32) (y u32))
                   (B)))
```

ISLE will generate the Rust type definition

```rust
#[derive(Clone, Debug)]
pub enum MyEnum {
    A { x: u32, y: u32, },
    B,
}
```

Note that enum variants with no fields take on the brace-less form,
while those with fields use the named-struct-field `A { x: ... }`
form. If all variants are field-less, then the type will additionally
derive `Copy`, `PartialEq`, and `Eq`.

If the type is declared as extern (`(decl MyEnum extern (enum ...))`)
then the same definition is assumed to exist. Primitives (`(decl u32
(primitive u32))`) are assumed to be defined already, and are required
to be `Copy`.

All imported/extern types are pulled in via `use super::*` at the top
of the generated code; thus, these types should exist in (or be
re-exported from) the parent module.

### Symbolic Constants

ISLE allows the user to refer to external constants as follows:

```lisp
    (extern const $I32 Type)
```

This allows code to refer to `$I32` whenever a value of type `Type` is
needed, in either a pattern (LHS) or an expression (RHS). These
constants are pulled in via the same `use super::*` that imports all
external types.

### Exported Interface: Functions and Context Trait

The generated ISLE code provides an interface that is designed to be
agnostic to the embedding application. This means that ISLE knows
nothing about, e.g., Cranelift or compiler concepts in
general. Rather, the generated code provides function entry points
with well-defined signatures based on the terms, and imports the
extern constructors and extractors via a context trait that the
embedder must implement.

When a term `T` is defined like

```lisp
    (decl T (A B C) U)
```

and has an internal constructor (provided by `rule` bodies), then a
function with the following signature will be exported from the
generated code:

```rust
    pub fn constructor_T<C: Context>(ctx: &mut C, arg0: &A, arg1: &B, arg2: &C) -> Option<U>;
```

In other words, `constructor_` is prepended, and the function takes
the expected arguments, along with a "context" (more on this
below). It returns an `Option<U>` because internal constructors can be
partial: if no rule's pattern matches, then the constructor
fails. Note that if a sub-constructor fails, no backtracking occurs;
rather, the failure propagates all the way up to the entry point.

What is this "context" for? The context argument is used to allow
external extractors and constructors to access the necessary state of
the embedding application. (For example, in Cranelift, it might be the
`LowerCtx` that controls the code-lowering process.)

The context is a trait because we want to decouple the generated code
from the application as much as possible. The trait will have a method
for each defined external extractor and constructor. For example, if
we have the following terms and declarations:

```lisp
    (decl A (u32 u32) T)
    (extern constructor A build_a)

    (decl B (T) U)
    (external extractor B disassemble_b)
```

then the `Context` trait will include these methods:

```rust
    trait Context {
        fn build_a(&mut self, arg0: u32, arg1: u32) -> T;
        fn disassemble_b(&mut self, arg0: &U) -> Option<T>;
    }
```

These functions should be implemented as described above for external
constructors and extractors.

Note that some external extractors are known to always succeed, for
example if they are just fetching some information that is always
present; in this case, the generated code can be made slightly more
efficient if we tell the ISLE compiler that this is so. By declaring

```lisp
    (external extractor infallible B disassemble_b)
```

we eliminate the `Option` on the return type, so the method is instead

```rust
    trait Context {
        // ...
        fn disassemble_b(&mut self, arg0: &U) -> T;
    }
```

## ISLE Internals

### Compiler Stages

Some detail and pointers to the compilation stages can be found in the
[README](../isle/README.md). The sequence starts as any ordinary
compiler: lexing, parsing, semantic analysis, and generation of an
IR. The most unique part is the "decision trie generation", which is
what converts the unordered-list-of-rule representation into something
that corresponds to the final Rust code's control flow and order of
matching operations.

We describe this data structure below with the intent to provide an
understanding of how the DSL compiler weaves rules together into Rust
control flow. While this understanding shouldn't be strictly necessary
to use the DSL, it may be helpful. (The ultimate answer to "how does
the generated code work" is, of course, found by reading the generated
code; some care has been taken to ensure it is reasonably legible for
human consumption!)

### Decision Trie

The heart of the ISLE transformation lies in how the compiler converts
a list of rules into a scheme to attempt to match rules in some order,
possibly sharing match operations between similar rules to reduce
work.

The core data structure we produce is a "decision trie" per internal
constructor body. This is an intermediate representation of sorts that
is built from individual-rule IR (LHS + RHS) sequences, and is then
used to generate Rust source.

The decision trie is, as the name implies, a kind of decision tree, in
the sense that we start at the root and move down the tree based on
the result of match operations (each feeding one "decision").

It is a "trie" (which is a kind of tree) because at each level, its
edges are labeled with match operations; a trie is a tree where one
input character from an alphabet is used to index children at each
level.

Each node in the tree is either an internal decision node, or a leaf
"expression" node (which we reach once we have a successful rule
match). The "execution semantics" of the trie are
backtracking-based. We attempt to find some path down the tree through
edges whose match ops run successfully; when we do this to reach a
leaf, we have the values generated by all of the match ops, and we can
execute the sequence of "expression instructions" in the leaf. Each
rule's left-hand side becomes a series of edges (merged into the
existing tree as we process rules) and each rule's right-hand side
becomes one leaf node with expression instructions.

At any point, if a match op does not succeed, we try the next out-edge
in sequence. If we have tried all out-edges from a decision node and
none were successful, then we backtrack one level further. Thus, we
simply perform an in-order tree traversal and find the first
successful match.

Though this sounds possibly very inefficient if some decision node has
a high fan-out, in practice it is not because the edges are often
known to be *mutually exclusive*. The canonical example of this is
when an enum-typed value is destructured into different variants by
various edges; we can use a Rust `match` statement in the generated
source and have `O(1)` (or close to it) cost for the dispatch at this
level.[^8]

Building the trie is a somewhat subtle procedure; see [this block
comment](https://github.com/bytecodealliance/wasmtime/blob/main/cranelift/isle/isle/src/trie.rs#L15-L166)
for more information regarding the trie construction algorithm.

[^8]: The worst-case complexity for a single term rewriting operation
      is still the cost of evaluating each rule's left-hand side
      sequentially, because in general there is no guarantee of
      overlap between the patterns. Ordering of the edges out of a
      decision node also affects complexity: if mutually-exclusive
      match operations are not adjacent, then they cannot be merged
      into a single `match` with `O(1)` dispatch. In general this
      ordering problem is quite difficult. We could do better with
      stronger heuristics; this is an open area for improvement in the
      DSL compiler!

## Reference: ISLE Language Grammar

Baseline: allow arbitrary whitespace, and wasm-style comments (`;` to
newline, or nested block-comments with `(;` and `;)`).

The grammar accepted by the parser is as follows:

```bnf
<skip> ::= <whitespace> | <comment>

<whitespace> ::= " "
               | "\t"
               | "\n"
               | "\r"

<comment> ::= <line-comment> | <block-comment>

<line-comment> ::= ";" <line-char>* (<newline> | eof)
<line-char> ::= <any character other than "\n" or "\r">
<newline> ::= "\n" | "\r"

<block-comment> ::= "(;" <block-char>* ";)"
<block-char> ::= <any character other than ";" or "(">
               | ";" if the next character is not ")"
               | "(" if the next character is not ";"
               | <block-comment>

<ISLE> ::= <def>*

<def> ::= "(" "pragma" <pragma> ")"
        | "(" "type" <typedecl> ")"
        | "(" "decl" <decl> ")"
        | "(" "rule" <rule> ")"
        | "(" "extractor" <extractor> ")"
        | "(" "extern" <extern> ")"
        | "(" "convert" <converter> ")"

;; No pragmas are defined yet
<pragma> ::= <ident>

<typedecl> ::= <ident> [ "extern" | "nodebug" ] <type-body>

<ident> ::= <ident-start> <ident-cont>*
<const-ident> ::= "$" <ident-cont>*
<ident-start> ::= <any non-whitespace character other than "-", "0".."9", "(", ")" or ";">
<ident-cont>  ::= <any non-whitespace character other than "(", ")", ";" or "@">

<type-body> ::= "(" "primitive" <ident> ")"
              | "(" "enum" <enum-variant>* ")"

<enum-variant> ::= <ident>
                 | "(" <ident> <variant-field>* ")"

<variant-field> ::= "(" <ident> <ty> ")"

<ty> ::= <ident>

<decl> ::= [ "pure" ] [ "multi" ] [ "partial" ] <ident> "(" <ty>* ")" <ty>

<rule> ::= [ <ident> ] [ <prio> ] <pattern> <stmt>* <expr>

<prio> ::= <int>

<int> ::= [ "-" ] ( "0".."9" | "_" )+
        | [ "-" ] "0" ("x" | "X") ( "0".."9" | "A".."F" | "a".."f" | "_" )+
        | [ "-" ] "0" ("o" | "O") ( "0".."7" | "_" )+
        | [ "-" ] "0" ("b" | "B") ( "0".."1" | "_" )+

<pattern> ::= <int>
            | "true" | "false"
            | <const-ident>
            | "_"
            | <ident>
            | <ident> "@" <pattern>
            | "(" "and" <pattern>* ")"
            | "(" <ident> <pattern>* ")"

<stmt> ::= "(" "if-let" <pattern> <expr> ")"
         | "(" "if" <expr> ")"

<expr> ::= <int>
         | "true" | "false"
         | <const-ident>
         | <ident>
         | "(" "let" "(" <let-binding>* ")" <expr> ")"
         | "(" <ident> <expr>* ")"

<let-binding> ::= "(" <ident> <ty> <expr> ")"

<extractor> ::= "(" <ident> <ident>* ")" <pattern>

<extern> ::= "constructor" <ident> <ident>
           | "extractor" [ "infallible" ] <ident> <ident>
           | "const" <const-ident> <ident> <ty>

<converter> ::= <ty> <ty> <ident>
```

## Reference: ISLE Language Grammar verification extensions
```bnf
<def> += "(" "spec" <spec> ")"
       | "(" "model" <model> ")"
       | "(" "form" <form> ")"
       | "(" "instantiate" <instantiation> ")"

<spec> ::= "(" <ident> <ident>* <provide> [ <require> ] ")"
<provide> ::= "(" "provide" <spec-expr>* ")"
<require> ::= "(" "require" <spec-expr>* ")"

<model> ::= <ty> "(" "type" <model-ty> ")"
          | <ty> "(" "enum" <model-variant>* ")"

<model-ty> ::= "Bool"
             | "Int"
             | "Unit"
             | "(" "bv" <int> ")"

<model-variant> ::= "(" <ident> [ <spec-expr> ]  ")"

<form> ::= <ident> <signature>*

<instantiation> ::= <ident> "(" <signature>* ")"
                  | <ident> <ident>

<spec-expr> ::= <int>
              | <spec-bv>
              | "true" | "false"
              | <ident>
              | "(" "switch" <spec-expr> <spec-pair>* ")"
              | "(" <spec-op> <spec-expr>* ")"
              | "(" <ident> ")"
              | "(" ")"

<spec-pair> ::= "(" <spec-expr> <spec-expr> ")"

<spec-op> ::= "and" | "not" | "or" | "=>"
            | "=" | "<=" | "<" | ">=" | ">"
            | "bvnot" | "bvand" | "bvor" | "bvxor"
            | "bvneg" | "bvadd" | "bvsub" | "bvmul"
            | "bvudiv" | "bvurem" | "bvsdiv" | "bvsrem"
            | "bvshl" | "bvlshr| | "bvashr"
            | "bvsaddo" | "subs"
            | "bvule" | "bvult" | "bvugt" | "bvuge"
            | "bvsle" | "bvslt" | "bvsgt" | "bvsge"
            | "rotr" | "rotl"
            | "extract" | "concat" | "conv_to"
            | "zero_ext" | "sign_ext"
            | "int2bv" | "bv2int"
            | "widthof"
            | "if" | "switch"
            | "popcnt" | "rev" | "cls" | "clz"
            | "load_effect" | "store_effect"

<signature>  ::= "(" <sig-args> <sig-ret> <sig-canon> ")"
<sig-args>   ::= "(" "args" <model-ty>* ")"
<sig-ret>    ::= "(" "ret" <model-ty>* ")"
<sig-canon>  ::= "(" "canon" <model-ty>* ")"
```
