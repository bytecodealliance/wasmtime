# Specification Language

Description of specification language.

## Types

ISLE types have a corresponding _model_ in the verification domain:

```
(model <isle_type> (type <type>))
```

Verification types `<type>` may be primitives, named, or compound types.

_Primitives_:

* Integer: `Int`
* Boolean: `Bool`
* Bit-vector: unknown width `(bv)`, fixed width `(bv <n>)`
* Unit: `Unit`
* Unspecified: `!`
* Auto: `_`, a primitive type to be deduced by type inference

> [!NOTE]
> The unspecified type exists to allow placeholder type specifications when a
> type must be specified to proceed, but it is irrelevant to the problem at hand.
> For example, an enum type may bring into scope variants with new types that are
> not important but need some specification.

_Named_ type references resolve to the same verification domain type model as
`<isle_type>`:

```
(named <isle_type>)
```

_Structs_ are purely structurally typed:

```
(struct
    (<field1> <type1>)
    (<field2> <type2>)
    ...
)
```

_Enum_ types exist in the verification domain but may only be inferred from
corresponding ISLE enum types. Custom enum types cannot be declared by users,
though it is allowed to override the inferred enum type for an ISLE enum with a
custom non-enum model.

## Specifications

Term specifications take the form:

```
(spec (<term> <params...>)
    (modifies <state> <cond>?)
    (provide <expr...>)
    (require <expr...>)
    (match <expr...>)
)
```

All `<expr...>` lists in the specification must be boolean and for each clause
multiple expressions are wrapped in an implicit `(and <exprs...>)`.

`(modifies <state> <cond>?)`:
concerns state modification, discussed in the "State" section below.

`(provide <expr...>)`:
post-conditions for the term. Post-conditions are assumed when the term appears
as a callee, and asserted when as a caller (root of rule expansion).

`(require <expr...>)`:
pre-conditions for the term. Pre-conditions are asserted when the term appears
as a callee, and assumed when as a caller (root of rule expansion).

`(match <expr...>)`:
may only be present on specs for _partial_ terms: non-infallible extractors or
partial constructors.  Partial terms may be thought of as implicitly returning
an `Option` type, and the match clause specifies the conditions under which the
return is `Some(..)`. In this case, the provide specification is conditioned on
the match specification holding.

Variables accessible to spec expressions depend on the term type and the clause.
For a term with parameters `(<term> <params...>)` and implicit result in special
`result` variable:

* Constructor: inputs are `[<params...>]`, outputs are `[result]`
* Extractor: inputs are `[result]`, inputs are `[<params...>]`

Variables in scope:

* Term inputs are available to all clauses.
* Term outputs are only available to the `provide` clause.
* State variables are global and available to all clauses.
* Modifies condition variables are available to all clauses.

### Expressions

Specification expressions may be:

**Constants:**
integer `<decimal>`, bitvector `#b<binary>` or `#x<hex>`, and booleans
`true`/`false`.

**Variables:**
plain identifiers refer to in-scope variables. Variables may refer to: term
parameters, the implicit `result` of a term, let or with bindings, macro
arguments, declared state, and state modification path conditions.

**Operators:**
operator applications of the form `(<op> <args...>)`. Available operators are
listed in the next section.

**Let bindings:**
let bindings introduce new variables with expression initializers, and evaluate
to a body expression that may reference the new variables brought into scope.
```
(let
    (
        (<v1> <init1>)
        (<v2> <init2>)
        ...
    )
    <body>
)
```
Let bindings may not shadow variables in the outer scope.

**With bindings:**
`with` expressions evaluate an expression with new _uninitialized_ variables
brought into scope.
```
(with (<v1> <v2> ...)
    <body>
)
```

**Field Access:**
the expression `(:<field> <x>)` accesses field `<field>` of the struct-valued expression `<x>`.

**Discriminator:**
expression `(?<variant> <x>)` evaluates to true if the enum-valued expression
`<x>` has the given variant.

**Variant Constructor:**
`(<enum>.<variant> <fields...>)` constructs an enum value with the given variant
and (optional) fields.

**Struct Constructor:**
`(struct (<field> <value>) ...)` constructs a struct value with the given fields.

**Match:**
the match operator pattern matches on enum types.

```
(match <on>
    ((<enum1>.<variant1> <fields1...>) <body1>)
    ((<enum2>.<variant2> <fields2...>) <body2>)
    ...
)
```

The value of the expression is the body of the arm that matches `<on>`,
evaluated with the fields brought into scope. If no arm matches the value is
undefined.

> [!WARNING]
> Under the hood `match` and `switch` are treated differently. Match is a
> top-level expression type, while `switch` is an operator. This makes no sense
> and should be fixed. It makes no difference to the user, however.

**Macro Expansion:**
`(<macro>! <args...>)` evaluates macro `<macro>` with the given arguments.

**Qualified Expressions:**
`(as <x> <ty>)` evaluates to `<x>` and provides a type inference annotation that
`<x>` must have type `<ty>`.

### Operators

Spec expression operators:

```
    // Boolean operations
    Eq,
    And,
    Or,
    Not,
    Imp,

    // Integer comparisons
    Lt,
    Lte,
    Gt,
    Gte,

    // Bitwise bitvector operations (directly SMT-LIB)
    BVNot,
    BVAnd,
    BVOr,
    BVXor,

    // Bitvector arithmetic operations  (directly SMT-LIB)
    BVNeg,
    BVAdd,
    BVSub,
    BVMul,
    BVUdiv,
    BVUrem,
    BVSdiv,
    BVSrem,
    BVShl,
    BVLshr,
    BVAshr,

    // Bitvector comparison operations  (directly SMT-LIB)
    BVUle,
    BVUlt,
    BVUgt,
    BVUge,
    BVSlt,
    BVSle,
    BVSgt,
    BVSge,

    // Bitvector overflow checks (SMT-LIB pending standardization)
    BVSaddo,

    // Desugared bitvector arithmetic operations
    Rotr,
    Rotl,
    Extract,
    ZeroExt,
    SignExt,
    Concat,

    // Floating point (IEEE 754-2008)
    FPPositiveInfinity,
    FPNegativeInfinity,
    FPPositiveZero,
    FPNegativeZero,
    FPNaN,
    FPAdd,
    FPSub,
    FPMul,
    FPDiv,
    FPMin,
    FPMax,
    FPNeg,
    FPSqrt,
    FPIsZero,
    FPIsInfinite,
    FPIsNaN,
    FPIsNegative,
    FPIsPositive,

    // Custom encodings
    Popcnt,
    Clz,
    Cls,
    Rev,

    // Conversion operations
    ConvTo,
    Int2BV,
    BV2Nat,
    WidthOf,

    // Control operations
    If,
    Switch,
```

### Macros

Spec macros may be declared:

```
(macro (<name> <params...>) <body>)
```

Macro expansions are of the form `(<name>! <args...>)`. The body of the macro is
evaluated in a scope with paramters set to argument values, and the result
substituted for the expansion expression.

## Type Instantiation

Possible type signatures for a term may be enumerated with `instantiate`:

```
(instantiate <term> <sigs...>)
```

where term signatures are of the form:

```
((args <types...>) (ret <type>))
```

Since some type instantiations are common, sets of signatures may be declared as `form`s:

```
(form <name> <sigs...>)
```

and then referenced as short-hand in an `instantiate` declaration:

```
(instantiate <term> <form>)
```

In verification, the cartesian product of all type instantantiations for all
present terms is considered. Many of the combinations will be ruled out by type
inference before proceeding to verification.

## State

State variables are declared with a type and default specification:

```
(state <name>
    (type <type>)
    (default <default>)
)
```

The `<type>` is a verification domain type as discussed above. The default spec
is an expression that must have boolean value. It is evaluated in a scope with
the state variable bound to variable `<name>`.

State variables are accessible as global variables from specs. The `modifies`
clause on specs determines the conditions under which the default spec is
applied:

`(modifies <state>)`:
declare that a spec unconditionally modifies the state variable `<state>`. The
default spec for `<state>` is disabled.

`(modifies <state> <cond>)`:
conditionally modify `<state>` with conditional variable `<cond>`.  In this
case, the corresponding spec must provide constraints that define when `<cond>`
holds, and implied constraints on `<state>` if it does.  The default spec for
`<state>` will only apply if `<cond>` is false. Unconditional state modification
is equivalent to conditional state modification with an assertion that `<cond>`
is always true.

In verification, all the conditional variables for a given state are collected `<cond1>`, `<cond2>` and the default spec is conditionally assumed:

```
(=> (not (or <cond1> <cond2> ...)) <default>)
```

## Attributes

Attributes may be applied to terms and rules:

```
(attr rule? <name> <kind>)
```

Without the `rule` keyword, it is assumed to be a term attribute.

Attribute kinds:

`(attr <term> (veri chain))`:
In verification, apply rule chaining to this term.  A term marked for chaining
may omit a specification. Instead, all possible applications of rules to this
term will be generated and verified.

`(attr rule <rule> (veri priority))`:
In verification, declare that the correctness of lower priority rules depends on
this rule not matching.

During rule expansion, any higher-priority overlapping rules that have the
priority tag will have their match conditions negated and added to the
verification conditions.

Note that care must be taken when using this tag: if the specification for the
match conditions of the higher priority rule are an over-approximation of
reality, then the assumptions made by lower priority rules will be an
under-approximation. In an extreme case this may cause the verifier to determine
the lower priority rule never applies. In a more subtle case, it could cause
bugs to be missed.

`(attr rule? <name> (tag <tag>))`:
Tag attributes allow for categorizing terms and rules. They have no semantic
meaning but are useful for filtering verification in the command-line and
presenting aggregate verification status.
