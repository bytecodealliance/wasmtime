# Rules for Writing Optimization Rules

For both correctness and compile speed, we must be careful with our rules. A lot
of it boils down to the fact that, unlike traditional e-graphs, our rules are
*directional*.

1. Rules should not rewrite to worse code: the right-hand side should be at
   least as good as the left-hand side or better.

   For example, the rule

       x => (add x 0)

   is disallowed, but swapping its left- and right-hand sides produces a rule
   that is allowed.

   Any kind of canonicalizing rule that intends to help subsequent rules match
   and unlock further optimizations (e.g. floating constants to the right side
   for our constant-propagation rules to match) must produce canonicalized
   output that is no worse than its noncanonical input.

   We assume this invariant as a heuristic to break ties between two
   otherwise-equal-cost expressions in various places, making up for some
   limitations of our explicit cost function.

2. Any rule that removes value-uses in its right-hand side that previously
   existed in its left-hand side MUST use `subsume`.

   For example, the rule

       (select 1 x y) => x

   MUST use `subsume`.

   This is required for correctness because, once a value-use is removed, some
   e-nodes in the e-class are more equal than others. There might be uses of `x`
   in a scope where `y` is not available, and so emitting `(select 1 x y)` in
   place of `x` in such cases would introduce uses of `y` where it is not
   defined.

   An exception to this rule is discarding constants, as they can be
   rematerialized anywhere without introducing correctness issues. For example,
   the (admittedly silly) rule `(select 1 x (iconst_u _)) => x` would be a good
   candidate for not using `subsume`, as it does not discard any non-constant
   values introduced in its LHS.

3. Avoid overly general rewrites like commutativity and associativity. Instead,
   prefer targeted instances of the rewrite (for example, canonicalizing adds
   where one operand is a constant such that the constant is always the add's
   second operand, rather than general commutativity for adds) or even writing
   the "same" optimization rule multiple times.

   For example, the commutativity in the first rule in the following snippet is
   bad because it will match even when the first operand is not an add:

       ;; Commute to allow `(foo (add ...) x)`, when we see it, to match.
       (foo x y) => (foo y x)

       ;; Optimize.
       (foo x (add ...)) => (bar x)

   Better is to commute only when we know that canonicalizing in this way will
   all definitely allow the subsequent optimization rule to match:

       ;; Canonicalize all adds to `foo`'s second operand.
       (foo (add ...) x) => (foo x (add ...))

       ;; Optimize.
       (foo x (add ...)) => (bar x)

   But even better in this case is to write the "same" optimization multiple
   times:

       (foo (add ...) x) => (bar x)
       (foo x (add ...)) => (bar x)

   The cost of rule-matching is amortized by the ISLE compiler, where as the
   intermediate result of each rewrite allocates new e-nodes and requires
   storage in the dataflow graph. Therefore, additional rules are cheaper than
   additional e-nodes.

   Commutativity and associativity in particular can cause huge amounts of
   e-graph bloat.

   One day we intend to extend ISLE with built-in support for commutativity, so
   we don't need to author the redundant commutations ourselves:
   https://github.com/bytecodealliance/wasmtime/issues/6128
