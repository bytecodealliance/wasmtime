//! A strongly-normalizing intermediate representation for ISLE rules.
use crate::error::{Error, Source, Span};
use crate::lexer::Pos;
use crate::sema::{self, RuleVisitor};
use crate::DisjointSets;
use std::collections::{hash_map::Entry, HashMap};

/// A field index in a tuple or an enum variant.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct TupleIndex(u8);
/// A hash-consed identifier for a binding, stored in a [RuleSet].
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct BindingId(u16);
/// A hash-consed identifier for an expression, stored in a [RuleSet].
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ExprId(u16);

impl BindingId {
    /// Get the index of this id.
    pub fn index(self) -> usize {
        self.0.into()
    }
}

impl ExprId {
    /// Get the index of this id.
    pub fn index(self) -> usize {
        self.0.into()
    }
}

/// Binding sites are the result of Rust pattern matching.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Binding {
    /// A match begins at the result of some expression that produces a Rust value.
    Expr {
        /// Which expression is being matched?
        constructor: ExprId,
    },
    /// After some sequence of matches, we'll match one of the previous bindings against an enum
    /// variant and produce a new binding from one of its fields. There must be a matching
    /// [Constraint] for each `source`/`variant` pair that appears in a binding.
    Variant {
        /// Which binding is being matched?
        source: BindingId,
        /// Which enum variant are we pulling binding sites from? This is somewhat redundant with
        /// information in a corresponding [Constraint]. However, it must be here so that different
        /// enum variants aren't hash-consed into the same binding site.
        variant: sema::VariantId,
        /// Which field of this enum variant are we projecting out? Although ISLE uses named fields,
        /// we track them by index for constant-time comparisons. The [sema::TypeEnv] can be used to
        /// get the field names.
        field: TupleIndex,
    },
    /// After some sequence of matches, we'll match one of the previous bindings against
    /// `Option::Some` and produce a new binding from its contents. (This currently only happens
    /// with external extractors.)
    Some {
        /// Which binding is being matched?
        source: BindingId,
    },
    /// After some sequence of matches, we'll match one of the previous bindings against a tuple and
    /// produce a new binding from one of its fields. (This currently only happens with external
    /// extractors.)
    Tuple {
        /// Which binding is being matched?
        source: BindingId,
        /// Which tuple field are we projecting out?
        field: TupleIndex,
    },
}

/// Pattern matches which can fail.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Constraint {
    /// The value must match this enum variant.
    Variant(TupleIndex, sema::TypeId, sema::VariantId),
    /// The value must equal this integer literal.
    ConstInt(i128),
    /// The value must equal this Rust primitive value.
    ConstPrim(sema::Sym),
    /// The value must be an `Option::Some`, from a fallible extractor.
    Some,
}

/// Expressions construct new values. Rust pattern matching can only destructure existing values,
/// not call functions or construct new values. So `if-let` and external extractor invocations need
/// to interrupt pattern matching in order to evaluate a suitable expression. These expressions are
/// also used when evaluating the right-hand side of a rule.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum Expr {
    /// Evaluates to the given integer literal.
    ConstInt(i128),
    /// Evaluates to the given primitive Rust value.
    ConstPrim(sema::Sym),
    /// One of the arguments to the top-level function.
    Argument(TupleIndex),
    /// A binding from some sequence of pattern matches.
    Binding(BindingId),
    /// The result of calling an external extractor.
    Extractor(sema::TermId, ExprId),
    /// The result of constructing an enum variant.
    Variant(sema::TypeId, sema::VariantId, Box<[ExprId]>),
    /// The result of calling an external constructor.
    Constructor(sema::TermId, Box<[ExprId]>),
}

/// A term-rewriting rule. All [BindingId]s and [ExprId]s are only meaningful in the context of the
/// [RuleSet] that contains this rule.
#[derive(Debug, Default)]
pub struct Rule {
    /// Where was this rule defined?
    pub pos: Pos,
    /// All of these bindings must match for this rule to apply. Note that within a single rule, if
    /// a binding site must match two different constants, then the rule can never match.
    constraints: HashMap<BindingId, Constraint>,
    /// Sets of bindings which must be equal for this rule to match.
    pub equals: DisjointSets<BindingId>,
    /// If other rules apply along with this one, the one with the highest numeric priority is
    /// evaluated. If multiple applicable rules have the same priority, that's an overlap error.
    pub prio: i64,
    /// If this rule applies, the top-level term should evaluate to this expression.
    pub result: ExprId,
}

/// Records whether a given pair of rules can both match on some input.
pub enum Overlap {
    /// There is no input on which this pair of rules can both match.
    No,
    /// There is at least one input on which this pair of rules can both match.
    Yes {
        /// True if every input accepted by one rule is also accepted by the other. This does not
        /// indicate which rule is more general and in fact the rules could match exactly the same
        /// set of inputs. You can work out which by comparing the number of constraints in both
        /// rules: The more general rule has fewer constraints.
        subset: bool,
    },
}

/// A collection of [Rule]s, along with hash-consed [Binding]s and [Expr]s for all of them.
#[derive(Debug, Default)]
pub struct RuleSet {
    /// The [Rule]s for a single [sema::Term].
    pub rules: Vec<Rule>,
    /// The bindings identified by [BindingId]s within rules.
    pub bindings: Vec<Binding>,
    /// The expressions identified by [ExprId]s within rules.
    pub exprs: Vec<Expr>,
}

/// Construct a [RuleSet] for each term in `termenv` that has rules.
pub fn build(
    termenv: &sema::TermEnv,
    tyenv: &sema::TypeEnv,
) -> (Vec<(sema::TermId, RuleSet)>, Vec<Error>) {
    let mut errors = Vec::new();
    let mut term = HashMap::new();
    for rule in termenv.rules.iter() {
        term.entry(rule.lhs.root_term().unwrap())
            .or_insert_with(RuleSetBuilder::default)
            .add_rule(rule, termenv, tyenv, &mut errors);
    }

    // The `term` hash map may return terms in any order. Sort them to ensure that we produce the
    // same output every time when given the same ISLE source. Rules are added to terms in `RuleId`
    // order, so it's not necessary to sort within a `RuleSet`.
    let mut result: Vec<_> = term
        .into_iter()
        .map(|(term, builder)| (term, builder.rules))
        .collect();
    result.sort_unstable_by_key(|(term, _)| *term);

    (result, errors)
}

impl Rule {
    /// Returns whether a given pair of rules can both match on some input, and if so, whether
    /// either matches a subset of the other's inputs. If this function returns `No`, then the two
    /// rules definitely do not overlap. However, it may return `Yes` in cases where the rules can't
    /// overlap in practice, or where this analysis is not yet precise enough to decide.
    pub fn may_overlap(&self, other: &Rule) -> Overlap {
        // Two rules can't overlap if, for some binding site in the intersection of their
        // constraints, the rules have different constraints: an input can't possibly match both
        // rules then. If the rules do overlap, and one has a subset of the constraints of the
        // other, then the less-constrained rule matches every input that the more-constrained rule
        // matches, and possibly more. We test for both conditions at once, with the observation
        // that if the intersection of two sets is equal to the smaller set, then it's a subset. So
        // the outer loop needs to go over the rule with fewer constraints in order to correctly
        // identify if it's a subset of the other rule. Also, that way around is faster.
        let (small, big) = if self.constraints.len() <= other.constraints.len() {
            (self, other)
        } else {
            (other, self)
        };

        // TODO: nonlinear constraints complicate the subset check
        // For the purpose of overlap checking, equality constraints act like other constraints, in
        // that they can cause rules to not overlap. However, because we don't have a concrete
        // pattern to compare, the analysis to prove that is complicated. For now, we approximate
        // the result. If `small` has any of these nonlinear constraints, conservatively report that
        // it is not a subset of `big`.
        let mut subset = small.equals.is_empty();

        for (binding, a) in small.constraints.iter() {
            if let Some(b) = big.constraints.get(binding) {
                if a != b {
                    // If any binding site is constrained differently by both rules then there is
                    // no input where both rules can match.
                    return Overlap::No;
                }
                // Otherwise both are constrained in the same way at this binding site. That doesn't
                // rule out any possibilities for what inputs the rules accept.
            } else {
                // The `big` rule's inputs are a subset of the `small` rule's inputs if every
                // constraint in `small` is exactly matched in `big`. But we found a counterexample.
                subset = false;
            }
        }
        Overlap::Yes { subset }
    }

    /// Returns the constraint that the given binding site must satisfy for this rule to match, if
    /// there is one.
    pub fn get_constraint(&self, source: BindingId) -> Option<Constraint> {
        self.constraints.get(&source).copied()
    }

    fn set_constraint(
        &mut self,
        source: BindingId,
        constraint: Constraint,
    ) -> Result<(), UnreachableError> {
        match self.constraints.entry(source) {
            Entry::Occupied(entry) => {
                if entry.get() != &constraint {
                    return Err(UnreachableError {
                        pos: self.pos,
                        constraint_a: *entry.get(),
                        constraint_b: constraint,
                    });
                }
            }
            Entry::Vacant(entry) => {
                entry.insert(constraint);
            }
        }
        Ok(())
    }
}

#[derive(Debug)]
struct UnreachableError {
    pos: Pos,
    constraint_a: Constraint,
    constraint_b: Constraint,
}

#[derive(Debug, Default)]
struct RuleSetBuilder {
    current_rule: Rule,
    binding_map: HashMap<Binding, BindingId>,
    expr_map: HashMap<Expr, ExprId>,
    unreachable: Vec<UnreachableError>,
    rules: RuleSet,
}

impl RuleSetBuilder {
    fn add_rule(
        &mut self,
        rule: &sema::Rule,
        termenv: &sema::TermEnv,
        tyenv: &sema::TypeEnv,
        errors: &mut Vec<Error>,
    ) {
        self.current_rule.pos = rule.pos;
        self.current_rule.prio = rule.prio;
        self.current_rule.result = rule.visit(self, termenv);
        self.normalize_equivalence_classes();
        let rule = std::mem::take(&mut self.current_rule);

        if self.unreachable.is_empty() {
            self.rules.rules.push(rule);
        } else {
            // If this rule can never match, drop it so it doesn't affect overlap checking.
            errors.extend(self.unreachable.drain(..).map(|err| {
                let src = Source::new(
                    tyenv.filenames[err.pos.file].clone(),
                    tyenv.file_texts[err.pos.file].clone(),
                );
                Error::UnreachableError {
                    msg: format!(
                        "rule requires binding to match both {:?} and {:?}",
                        err.constraint_a, err.constraint_b
                    ),
                    src,
                    span: Span::new_single(err.pos),
                }
            }))
        }
    }

    /// Establish the invariant that a binding site can have a concrete constraint or a nonlinear
    /// constraint, but not both. This is useful because overlap checking is most effective on
    /// concrete constraints, and also because it exposes more rule structure for codegen.
    ///
    /// If a binding site is constrained and also required to be equal to another binding site, then
    /// copy the constraint and push the equality inside it. For example:
    /// - `(term x @ 2 x)` is rewritten to `(term 2 2)`
    /// - `(term x @ (T.A _) x)` is rewritten to `(term (T.A y) (T.A y))`
    ///
    /// If several binding sites are supposed to be equal but they each have conflicting constraints
    /// then this rule is unreachable. For example, `(term x @ 2 (and x 3))` requires both arguments
    /// to be equal but also requires them to match both 2 and 3, which can't happen for any input.
    fn normalize_equivalence_classes(&mut self) {
        // First, find all the constraints that need to be copied to other binding sites in their
        // respective equivalence classes. Note: do not remove these constraints here! Yes, we'll
        // put them back later, but we rely on still having them around so that
        // `set_constraint_or_error` can detect conflicting constraints.
        let mut deferred_constraints = Vec::new();
        for (&binding, &constraint) in self.current_rule.constraints.iter() {
            if let Some(root) = self.current_rule.equals.find_mut(binding) {
                deferred_constraints.push((root, constraint));
            }
        }

        // Pick one constraint and propagate it through its equivalence class. If there are no
        // errors then it doesn't matter what order we do this in, because that means that any
        // redundant constraints on an equivalence class were equal. We can write equal values into
        // the constraint map in any order and get the same result. If there were errors, we aren't
        // going to generate code from this rule, so order only affects how conflicts are reported.
        while let Some((current, constraint)) = deferred_constraints.pop() {
            // Remove the entire equivalence class and instead add copies of this constraint to
            // every binding site in the class. If there are constraints on other binding sites in
            // this class, then when we try to copy this constraint to those binding sites,
            // `set_constraint_or_error` will check that the constraints are equal and record an
            // appropriate error otherwise.
            //
            // Later, we'll re-visit those other binding sites because they're still in
            // `deferred_constraints`, but `set` will be empty because we already deleted the
            // equivalence class the first time we encountered it.
            let set = self.current_rule.equals.remove_set_of(current);
            for &binding in set.iter() {
                self.set_constraint_or_error(binding, constraint);
            }

            match (constraint, set.split_first()) {
                // If the equivalence class was empty we don't have to do anything.
                (_, None) => {}

                // If we removed an equivalence class with an enum variant constraint, make the
                // fields of the variant equal instead. Create a binding for every field of every
                // member of `set`. Arbitrarily pick one to set all the others equal to.
                (Constraint::Variant(fields, _, variant), Some((&base, rest))) => {
                    let base_fields =
                        self.field_bindings(base, fields, variant, &mut deferred_constraints);
                    for &binding in rest {
                        for (&x, &y) in self
                            .field_bindings(binding, fields, variant, &mut deferred_constraints)
                            .iter()
                            .zip(base_fields.iter())
                        {
                            self.current_rule.equals.merge(x, y);
                        }
                    }
                }

                // These constraints don't introduce new binding sites.
                (Constraint::ConstInt(_) | Constraint::ConstPrim(_), _) => {}

                // Currently, `Some` constraints are only introduced implicitly during the
                // translation from `sema`, so there's no way to set the corresponding binding
                // sites equal to each other. Instead, any equality constraints get applied on
                // the results of matching `Some()` or tuple patterns.
                (Constraint::Some, _) => unreachable!(),
            }
        }
    }

    fn field_bindings(
        &mut self,
        binding: BindingId,
        fields: TupleIndex,
        variant: sema::VariantId,
        deferred_constraints: &mut Vec<(BindingId, Constraint)>,
    ) -> Box<[BindingId]> {
        (0..fields.0)
            .map(TupleIndex)
            .map(move |field| {
                let binding = self.dedup_binding(Binding::Variant {
                    source: binding,
                    variant,
                    field,
                });
                // We've just added an equality constraint to a binding site that may not have had
                // one already. If that binding site already had a concrete constraint, then we need
                // to "recursively" propagate that constraint through the new equivalence class too.
                if let Some(constraint) = self.current_rule.get_constraint(binding) {
                    deferred_constraints.push((binding, constraint));
                }
                binding
            })
            .collect()
    }

    fn dedup_binding(&mut self, binding: Binding) -> BindingId {
        if let Some(binding) = self.binding_map.get(&binding) {
            *binding
        } else {
            let id = BindingId(self.rules.bindings.len().try_into().unwrap());
            self.rules.bindings.push(binding.clone());
            self.binding_map.insert(binding, id);
            id
        }
    }

    fn dedup_expr(&mut self, expr: Expr) -> ExprId {
        if let Some(expr) = self.expr_map.get(&expr) {
            *expr
        } else {
            let id = ExprId(self.rules.exprs.len().try_into().unwrap());
            self.rules.exprs.push(expr.clone());
            self.expr_map.insert(expr, id);
            id
        }
    }

    fn set_constraint(&mut self, input: Binding, constraint: Constraint) -> BindingId {
        let input = self.dedup_binding(input);
        self.set_constraint_or_error(input, constraint);
        input
    }

    fn set_constraint_or_error(&mut self, input: BindingId, constraint: Constraint) {
        if let Err(e) = self.current_rule.set_constraint(input, constraint) {
            self.unreachable.push(e);
        }
    }
}

impl sema::PatternVisitor for RuleSetBuilder {
    type PatternId = Binding;

    fn add_match_equal(&mut self, a: Binding, b: Binding, _ty: sema::TypeId) {
        let a = self.dedup_binding(a);
        let b = self.dedup_binding(b);
        // If both bindings represent the same binding site, they're implicitly equal.
        if a != b {
            self.current_rule.equals.merge(a, b);
        }
    }

    fn add_match_int(&mut self, input: Binding, _ty: sema::TypeId, int_val: i128) {
        self.set_constraint(input, Constraint::ConstInt(int_val));
    }

    fn add_match_prim(&mut self, input: Binding, _ty: sema::TypeId, val: sema::Sym) {
        self.set_constraint(input, Constraint::ConstPrim(val));
    }

    fn add_match_variant(
        &mut self,
        input: Binding,
        input_ty: sema::TypeId,
        arg_tys: &[sema::TypeId],
        variant: sema::VariantId,
    ) -> Vec<Binding> {
        let fields = TupleIndex(arg_tys.len().try_into().unwrap());
        let source = self.set_constraint(input, Constraint::Variant(fields, input_ty, variant));
        (0..fields.0)
            .map(TupleIndex)
            .map(|field| Binding::Variant {
                source,
                variant,
                field,
            })
            .collect()
    }

    fn add_extract(
        &mut self,
        input: Binding,
        _input_ty: sema::TypeId,
        output_tys: Vec<sema::TypeId>,
        term: sema::TermId,
        infallible: bool,
        _multi: bool,
    ) -> Vec<Binding> {
        // External extractor invocations are expressions in Rust
        let input = self.pattern_as_expr(input);
        let input = self.dedup_expr(Expr::Extractor(term, input));
        let input = self.expr_as_pattern(input);

        // If the extractor is fallible, build a pattern and constraint for `Some`
        let source = if infallible {
            input
        } else {
            let source = self.set_constraint(input, Constraint::Some);
            Binding::Some { source }
        };

        // If the extractor has multiple outputs, create a separate binding for each
        match output_tys.len().try_into().unwrap() {
            0 => vec![],
            1 => vec![source],
            outputs => {
                let source = self.dedup_binding(source);
                (0..outputs)
                    .map(TupleIndex)
                    .map(|field| Binding::Tuple { source, field })
                    .collect()
            }
        }
    }
}

impl sema::ExprVisitor for RuleSetBuilder {
    type ExprId = ExprId;

    fn add_const_int(&mut self, _ty: sema::TypeId, val: i128) -> ExprId {
        self.dedup_expr(Expr::ConstInt(val))
    }

    fn add_const_prim(&mut self, _ty: sema::TypeId, val: sema::Sym) -> ExprId {
        self.dedup_expr(Expr::ConstPrim(val))
    }

    fn add_create_variant(
        &mut self,
        inputs: Vec<(ExprId, sema::TypeId)>,
        ty: sema::TypeId,
        variant: sema::VariantId,
    ) -> ExprId {
        self.dedup_expr(Expr::Variant(
            ty,
            variant,
            inputs.into_iter().map(|(expr, _)| expr).collect(),
        ))
    }

    fn add_construct(
        &mut self,
        inputs: Vec<(ExprId, sema::TypeId)>,
        _ty: sema::TypeId,
        term: sema::TermId,
        _infallible: bool,
        _multi: bool,
    ) -> ExprId {
        self.dedup_expr(Expr::Constructor(
            term,
            inputs.into_iter().map(|(expr, _)| expr).collect(),
        ))
    }
}

impl sema::RuleVisitor for RuleSetBuilder {
    type PatternVisitor = Self;
    type ExprVisitor = Self;
    type Expr = ExprId;

    fn add_arg(&mut self, index: usize, _ty: sema::TypeId) -> Binding {
        // Arguments don't need to be pattern-matched to reference them, so they're expressions
        let argument = TupleIndex(index.try_into().unwrap());
        let expr = self.dedup_expr(Expr::Argument(argument));
        Binding::Expr { constructor: expr }
    }

    fn add_pattern<F: FnOnce(&mut Self)>(&mut self, visitor: F) {
        visitor(self)
    }

    fn add_expr<F>(&mut self, visitor: F) -> ExprId
    where
        F: FnOnce(&mut Self) -> sema::VisitedExpr<Self>,
    {
        visitor(self).value
    }

    fn expr_as_pattern(&mut self, expr: ExprId) -> Binding {
        if let &Expr::Binding(binding) = &self.rules.exprs[expr.index()] {
            // Short-circuit wrapping a binding around an expr from another binding
            self.rules.bindings[binding.index()]
        } else {
            Binding::Expr { constructor: expr }
        }
    }

    fn pattern_as_expr(&mut self, pattern: Binding) -> ExprId {
        if let Binding::Expr { constructor } = pattern {
            // Short-circuit wrapping an expr around a binding from another expr
            constructor
        } else {
            let binding = self.dedup_binding(pattern);
            self.dedup_expr(Expr::Binding(binding))
        }
    }
}
