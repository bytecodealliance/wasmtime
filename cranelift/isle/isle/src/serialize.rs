//! Put "sea of nodes" representation of a `RuleSet` into a sequential order.
//!
//! We're trying to satisfy two key constraints on generated code:
//!
//! First, we must produce the same result as if we tested the left-hand side
//! of every rule in descending priority order and picked the first match.
//! But that would mean a lot of duplicated work since many rules have similar
//! patterns. We want to evaluate in an order that gets the same answer but
//! does as little work as possible.
//!
//! Second, some ISLE patterns can only be implemented in Rust using a `match`
//! expression (or various choices of syntactic sugar). Others can only
//! be implemented as expressions, which can't be evaluated while matching
//! patterns in Rust. So we need to alternate between pattern matching and
//! expression evaluation.
//!
//! To meet both requirements, we repeatedly partition the set of rules for a
//! term and build a tree of Rust control-flow constructs corresponding to each
//! partition. The root of such a tree is a [Block].
use std::cmp::Reverse;

use crate::lexer::Pos;
use crate::trie_again::{Binding, BindingId, Constraint, Overlap, Rule, RuleSet};
use crate::DisjointSets;

/// Decomposes the rule-set into a tree of [Block]s.
pub fn serialize(rules: &RuleSet) -> Block {
    let mut order = Vec::from_iter(0..rules.rules.len());
    Decomposition::new(rules).sort(&mut order)
}

/// A sequence of steps to evaluate in order. Any step may return early, so
/// steps ordered later can assume the negation of the conditions evaluated in
/// earlier steps.
#[derive(Default)]
pub struct Block {
    /// Steps to evaluate.
    pub steps: Vec<EvalStep>,
}

/// A step to evaluate involves possibly let-binding some expressions, then
/// executing some control flow construct.
pub struct EvalStep {
    /// Before evaluating this case, emit let-bindings in this order.
    pub bind_order: Vec<BindingId>,
    /// The control-flow construct to execute at this point.
    pub check: ControlFlow,
}

/// What kind of control-flow structure do we need to emit here?
pub enum ControlFlow {
    /// Test a binding site against one or more mutually-exclusive patterns and
    /// branch to the appropriate block if a pattern matches.
    Match {
        /// Which binding site are we examining at this point?
        source: BindingId,
        /// What patterns do we care about?
        arms: Vec<MatchArm>,
    },
    /// Test whether two binding sites have values which are equal when
    /// evaluated on the current input.
    Equal {
        /// One binding site.
        a: BindingId,
        /// The other binding site. To ensure we always generate the same code
        /// given the same set of ISLE rules, `b` should be strictly greater
        /// than `a`.
        b: BindingId,
        /// If the test succeeds, evaluate this block.
        body: Block,
    },
    /// Evaluate a block once with each value of the given binding site.
    Loop {
        /// A binding site of type [Binding::Iterator]. Its source binding site
        /// must be a multi-extractor or multi-constructor call.
        result: BindingId,
        /// What to evaluate with each binding.
        body: Block,
    },
    /// Return a result from the right-hand side of a rule. If we're building a
    /// multi-constructor then this doesn't actually return, but adds to a list
    /// of results instead. Otherwise this return stops evaluation before any
    /// later steps.
    Return {
        /// Where was the rule defined that had this right-hand side?
        pos: Pos,
        /// What is the result expression which should be returned if this
        /// rule matched?
        result: BindingId,
    },
}

/// One concrete pattern and the block to evaluate if the pattern matches.
pub struct MatchArm {
    /// The pattern to match.
    pub constraint: Constraint,
    /// If this pattern matches, it brings these bindings into scope. If a
    /// binding is unused in this block, then the corresponding position in the
    /// pattern's bindings may be `None`.
    pub bindings: Vec<Option<BindingId>>,
    /// Steps to evaluate if the pattern matched.
    pub body: Block,
}

/// Given a set of rules that's been partitioned into two groups, move rules
/// from the first partition to the second if there are higher-priority rules
/// in the second group. In the final generated code, we'll check the rules
/// in the first ("selected") group before any in the second ("deferred")
/// group. But we need the result to be _as if_ we checked the rules in strict
/// descending priority order.
///
/// When evaluating the relationship between one rule in the selected set and
/// one rule in the deferred set, there are two cases where we can keep a rule
/// in the selected set:
/// - The deferred rule is lower priority than the selected rule; or
/// - The two rules don't overlap, meaning they can't match on the same inputs.
/// In either case, if the selected rule matches then we know the deferred rule
/// would not have been the one we wanted anyway; and if it doesn't match then
/// the fall-through semantics of the code we generate will let us go on to
/// check the deferred rule.
///
/// So a rule can stay in the selected set as long as it's in one of the above
/// relationships with every rule in the deferred set.
///
/// In practice only the priority matters. Checking overlap here doesn't change
/// the output on any backend as of this writing. But I've measured and it
/// hardly takes any time, so I'm leaving it in just in case somebody writes
/// rules someday where it helps.
fn respect_priority(rules: &RuleSet, order: &mut [usize], partition_point: usize) -> usize {
    let (selected, deferred) = order.split_at_mut(partition_point);
    if deferred.is_empty() {
        // In this case, `deferred.iter().all()` below will always return
        // `true`, so `partition_in_place` will keep everything in the first
        // partition. Short-circuit that.
        return partition_point;
    }

    partition_in_place(selected, |&idx| {
        let rule = &rules.rules[idx];
        deferred.iter().all(|&idx| {
            let other = &rules.rules[idx];
            // Overlap checking takes some work, so check priority first. And
            // if two rules have the same priority, we can assume they don't
            // overlap since otherwise the earlier overlap checking phase would
            // have already rejected this rule set.
            rule.prio >= other.prio || rule.may_overlap(other) == Overlap::No
        })
    })
}

/// A query which can be tested against a [trie_again::Rule] to see if that
/// rule requires the given kind of control flow around the given binding
/// sites. These choices correspond to the identically-named variants of
/// [ControlFlow].
///
/// The order of these variants is significant, because it's used as a tie-
/// breaker in the heuristic that picks which control flow to generate next.
///
/// - Loops should always be chosen last. If a rule needs to run once for each
///   value from an iterator, but only if some other condition is true, we
///   should check the other condition first.
///
/// - Sorting concrete [HasControlFlow::Match] constraints first has the effect
///   of clustering such constraints together, which is not important but means
///   codegen could theoretically merge the cluster of matches into a single
///   Rust `match` statement.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum HasControlFlow {
    /// Find rules which have a concrete pattern constraint on the given
    /// binding site.
    Match(BindingId),

    /// Find rules which require both given binding sites to be in the same
    /// equivalence class. If the same binding site is specified twice, then
    /// for a single rule, either that site is not in any equivalence class, or
    /// it obviously is in the same equivalence class as itself. This special
    /// case is useful for finding rules which have any equality constraint at
    /// all that involves the given binding site.
    Equal(BindingId, BindingId),

    /// Find rules which must loop over the multiple values of the given
    /// binding site.
    Loop(BindingId),
}

impl HasControlFlow {
    /// Identify which rules satisfy this query. Partition matching rules
    /// first in `order`, and return the number of rules found. No ordering
    /// is guaranteed within either partition, which allows this function to
    /// run in linear time. That's fine because later we'll recursively sort
    /// both partitions.
    fn partition_ignoring_priority(self, rules: &RuleSet, order: &mut [usize]) -> usize {
        partition_in_place(order, |&idx| {
            let rule = &rules.rules[idx];
            match self {
                HasControlFlow::Match(binding_id) => rule.get_constraint(binding_id).is_some(),
                HasControlFlow::Equal(x, y) => {
                    let x = rule.equals.find(x);
                    let y = rule.equals.find(y);
                    x.zip(y).filter(|(x, y)| x == y).is_some()
                }
                HasControlFlow::Loop(binding_id) => rule.iterators.contains(&binding_id),
            }
        })
    }

    /// Identify rules which both:
    /// 1. satisfy this query, like
    ///    [HasControlFlow::partition_ignoring_priority], and
    /// 2. are safe to evaluate before all rules that don't satisfy the query,
    ///    considering rules' relative priorities, like [respect_priority].
    /// This combination is usually what you want, but sometimes it's useful to
    /// check these conditions separately.
    fn partition(self, rules: &RuleSet, order: &mut [usize]) -> usize {
        let constrained = self.partition_ignoring_priority(rules, order);
        respect_priority(rules, order, constrained)
    }
}

/// As we proceed through sorting a term's rules, the term's binding sites move
/// through this sequence of states. This state machine helps us avoid doing
/// the same thing with a binding site more than once in any subtree.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum BindingState {
    /// Initially, all binding sites are unavailable for evaluation except for
    /// top-level arguments, constants, and similar.
    Unavailable,
    /// As more binding sites become available, it becomes possible to evaluate
    /// bindings which depend on those sites.
    Available,
    /// Once we've decided a binding is needed in order to make progress in
    /// matching, we emit a let-binding for it. We shouldn't evaluate it a
    /// second time, if possible.
    Emitted,
    /// We can only match a constraint against a binding site if we can emit it
    /// first. Afterward, we should not try to match a constraint against that
    /// site again in the same subtree.
    Matched,
}

/// A rule filter ([HasControlFlow]), plus temporary storage for the sort
/// key used in `best_control_flow` to order these candidates. Keeping the
/// temporary storage here lets us avoid repeated heap allocations.
#[derive(Clone, Debug, Eq, PartialEq)]
struct Candidate {
    kind: HasControlFlow,
    count: usize,
    state: BindingState,
}

impl Candidate {
    /// Construct a candidate where the sort key is smaller than that
    /// of any valid candidate. The sort key will need to be reset by
    /// `best_control_flow` before use.
    const fn with_invalid_key(kind: HasControlFlow) -> Self {
        Candidate {
            kind,
            count: 0,
            state: BindingState::Unavailable,
        }
    }
}

impl PartialOrd for Candidate {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Candidate {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // We prefer to match as many rules at once as possible.
        let base = self.count.cmp(&other.count);
        // Break ties by preferring bindings we've already emitted.
        base.then(self.state.cmp(&other.state))
            // Last resort tie-breaker: defer to HasControlFlow order, but
            // prefer control-flow that sorts earlier.
            .then(other.kind.cmp(&self.kind))
    }
}

/// Builder for one [Block] in the tree.
struct Decomposition<'a> {
    /// The complete RuleSet, shared across the whole tree.
    rules: &'a RuleSet,
    /// The state of all binding sites at this point in the tree, indexed by
    /// [BindingId]. Cloned when decomposing children of this block so they
    /// can reflect the state in nested scopes without falsely claiming that
    /// bindings are subsequently available in this outer scope.
    ready: Vec<BindingState>,
    /// The current set of candidates for control flow to add at this point in
    /// the tree. These are sorted before use, so order doesn't matter. Cloned
    /// when decomposing children of this block because we can't rely on any
    /// match results that might be computed in a nested scope, so if we still
    /// care about a candidate in the fallback case then we need to emit the
    /// correct control flow for it again.
    candidates: Vec<Candidate>,
    /// Equivalence classes that we've established on the current path from the
    /// root. Cloned when decomposing children of this block because each child
    /// has a different scope.
    equal: DisjointSets<BindingId>,
    /// Accumulator for bindings that should be emitted before the next
    /// control-flow construct.
    bind_order: Vec<BindingId>,
    /// Accumulator for the final Block that we'll return as this subtree.
    block: Block,
}

impl<'a> Decomposition<'a> {
    /// Create a builder for the root [Block].
    fn new(rules: &'a RuleSet) -> Decomposition<'a> {
        let mut result = Decomposition {
            rules,
            ready: vec![BindingState::Unavailable; rules.bindings.len()],
            candidates: Default::default(),
            equal: Default::default(),
            bind_order: Default::default(),
            block: Default::default(),
        };
        result.add_bindings();
        result
    }

    /// Create a builder for a nested [Block].
    fn new_block(&mut self) -> Decomposition {
        Decomposition {
            rules: self.rules,
            ready: self.ready.clone(),
            candidates: self.candidates.clone(),
            equal: self.equal.clone(),
            bind_order: Default::default(),
            block: Default::default(),
        }
    }

    /// Ensure that every binding site's state reflects its dependencies'
    /// states. This takes time linear in the number of bindings. Because
    /// `trie_again` only hash-conses a binding after all its dependencies have
    /// already been hash-consed, a single in-order pass visits a binding's
    /// dependencies before visiting the binding itself.
    fn add_bindings(&mut self) {
        for (idx, binding) in self.rules.bindings.iter().enumerate() {
            // We only add these bindings when matching a corresponding
            // type of control flow, in `make_control_flow`.
            if matches!(
                binding,
                Binding::Iterator { .. } | Binding::MatchVariant { .. } | Binding::MatchSome { .. }
            ) {
                continue;
            }

            // TODO: proactively put some bindings in `Emitted` state
            // That makes them visible to the best-binding heuristic, which
            // prefers to match on already-emitted bindings first. This helps
            // to sort cheap computations before expensive ones.

            let idx: BindingId = idx.try_into().unwrap();
            if self.ready[idx.index()] < BindingState::Available {
                if binding
                    .sources()
                    .iter()
                    .all(|&source| self.ready[source.index()] >= BindingState::Available)
                {
                    self.set_ready(idx, BindingState::Available);
                }
            }
        }
    }

    /// Determines the final evaluation order for the given subset of rules, and
    /// builds a [Block] representing that order.
    fn sort(mut self, mut order: &mut [usize]) -> Block {
        while let Some(best) = self.best_control_flow(order) {
            // Peel off all rules that have this particular control flow, and
            // save the rest for the next iteration of the loop.
            let partition_point = best.partition(&self.rules, order);
            debug_assert!(partition_point > 0);
            let (this, rest) = order.split_at_mut(partition_point);
            order = rest;

            // Recursively build the control-flow tree for these rules.
            let check = self.make_control_flow(best, this);
            // Note that `make_control_flow` may have added more let-bindings.
            let bind_order = std::mem::take(&mut self.bind_order);
            self.block.steps.push(EvalStep { bind_order, check });
        }

        // At this point, `best_control_flow` says the remaining rules don't
        // have any control flow left to emit. That could be because there are
        // no unhandled rules left, or because every candidate for control flow
        // for the remaining rules has already been matched by some ancestor in
        // the tree.
        debug_assert!(self.candidates.iter().all(|c| {
            if let &Candidate {
                kind: HasControlFlow::Equal(x, y),
                ..
            } = c
            {
                let x = self.equal.find(x);
                let y = self.equal.find(y);
                x.zip(y).filter(|(x, y)| x == y).is_some()
            } else {
                false
            }
        }));

        // If we're building a multi-constructor, then there could be multiple
        // rules with the same left-hand side. We'll evaluate them all, but
        // to keep the output consistent, first sort by descending priority
        // and break ties with the order the rules were declared. In non-multi
        // constructors, there should be at most one rule remaining here.
        order.sort_unstable_by_key(|&idx| (Reverse(self.rules.rules[idx].prio), idx));
        for &idx in order.iter() {
            let &Rule {
                pos,
                result,
                ref impure,
                ..
            } = &self.rules.rules[idx];

            // Ensure that any impure constructors are called, even if their
            // results aren't used.
            for &impure in impure.iter() {
                self.use_expr(impure);
            }
            self.use_expr(result);

            let check = ControlFlow::Return { pos, result };
            let bind_order = std::mem::take(&mut self.bind_order);
            self.block.steps.push(EvalStep { bind_order, check });
        }

        self.block
    }

    /// Let-bind this binding site and all its dependencies, skipping any
    /// which are already let-bound. Also skip let-bindings for certain trivial
    /// expressions which are safe and cheap to evaluate multiple times,
    /// because that reduces clutter in the generated code.
    fn use_expr(&mut self, name: BindingId) {
        if self.ready[name.index()] < BindingState::Emitted {
            self.set_ready(name, BindingState::Emitted);
            let binding = &self.rules.bindings[name.index()];
            for &source in binding.sources() {
                self.use_expr(source);
            }

            let should_let_bind = match binding {
                Binding::ConstInt { .. } => false,
                Binding::ConstPrim { .. } => false,
                Binding::Argument { .. } => false,
                Binding::MatchTuple { .. } => false,

                // Only let-bind variant constructors if they have some fields.
                // Building a variant with no fields is cheap, but don't
                // duplicate more complex expressions.
                Binding::MakeVariant { fields, .. } => !fields.is_empty(),

                // By default, do let-bind: that's always safe.
                _ => true,
            };
            if should_let_bind {
                self.bind_order.push(name);
            }
        }
    }

    /// Build one control-flow construct and its subtree for the specified rules.
    /// The rules in `order` must all have the kind of control-flow named in `best`.
    fn make_control_flow(&mut self, best: HasControlFlow, order: &mut [usize]) -> ControlFlow {
        match best {
            HasControlFlow::Match(source) => {
                self.use_expr(source);
                self.add_bindings();
                let mut arms = Vec::new();

                let get_constraint =
                    |idx: usize| self.rules.rules[idx].get_constraint(source).unwrap();

                // Ensure that identical constraints are grouped together, then
                // loop over each group.
                order.sort_unstable_by_key(|&idx| get_constraint(idx));
                for g in group_by_mut(order, |&a, &b| get_constraint(a) == get_constraint(b)) {
                    // Applying a constraint moves the discriminant from
                    // Emitted to Matched, but only within the constraint's
                    // match arm; later fallthrough cases may need to match
                    // this discriminant again. Since `source` is in the
                    // `Emitted` state in the parent due to the above call
                    // to `use_expr`, calling `add_bindings` again after this
                    // wouldn't change anything.
                    let mut child = self.new_block();
                    child.set_ready(source, BindingState::Matched);

                    // Get the constraint for this group, and all of the
                    // binding sites that it introduces.
                    let constraint = get_constraint(g[0]);
                    let bindings = Vec::from_iter(
                        constraint
                            .bindings_for(source)
                            .into_iter()
                            .map(|b| child.rules.find_binding(&b)),
                    );

                    let mut changed = false;
                    for &binding in bindings.iter() {
                        if let Some(binding) = binding {
                            // Matching a pattern makes its bindings
                            // available, and also emits code to bind
                            // them.
                            child.set_ready(binding, BindingState::Emitted);
                            changed = true;
                        }
                    }

                    // As an optimization, only propagate availability
                    // if we changed any binding's readiness.
                    if changed {
                        child.add_bindings();
                    }

                    // Recursively construct a Block for this group of rules.
                    let body = child.sort(g);
                    arms.push(MatchArm {
                        constraint,
                        bindings,
                        body,
                    });
                }

                ControlFlow::Match { source, arms }
            }

            HasControlFlow::Equal(a, b) => {
                // Both sides of the equality test must be evaluated before
                // the condition can be tested. Go ahead and let-bind them
                // so they're available without re-evaluation in fall-through
                // cases.
                self.use_expr(a);
                self.use_expr(b);
                self.add_bindings();

                let mut child = self.new_block();
                // Never mark binding sites used in equality constraints as
                // "matched", because either might need to be used again in
                // a later equality check. Instead record that they're in the
                // same equivalence class on this path.
                child.equal.merge(a, b);
                let body = child.sort(order);
                ControlFlow::Equal { a, b, body }
            }

            HasControlFlow::Loop(source) => {
                // Consuming a multi-term involves two binding sites:
                // calling the multi-term to get an iterator (the `source`),
                // and looping over the iterator to get a binding for each
                // `result`.
                let result = self
                    .rules
                    .find_binding(&Binding::Iterator { source })
                    .unwrap();

                // We must not let-bind the iterator until we're ready to
                // consume it, because it can only be consumed once. This also
                // means that the let-binding for `source` is not actually
                // reusable after this point, so even though we need to emit
                // its let-binding here, we pretend we haven't.
                let base_state = self.ready[source.index()];
                debug_assert_eq!(base_state, BindingState::Available);
                self.use_expr(source);
                self.ready[source.index()] = base_state;
                self.add_bindings();

                let mut child = self.new_block();
                child.set_ready(source, BindingState::Matched);
                child.set_ready(result, BindingState::Emitted);
                child.add_bindings();
                let body = child.sort(order);
                ControlFlow::Loop { result, body }
            }
        }
    }

    /// Advance the given binding to a new state. The new state usually should
    /// be greater than the existing state; but at the least it must never
    /// go backward.
    fn set_ready(&mut self, source: BindingId, state: BindingState) {
        let old = &mut self.ready[source.index()];
        debug_assert!(*old <= state);

        // Add candidates for this binding, but only when it first becomes
        // available.
        if let BindingState::Unavailable = old {
            // A binding site can't have all of these kinds of constraint,
            // and many have none. But `best_control_flow` has to check all
            // candidates anyway, so let it figure out which (if any) of these
            // are applicable. It will only check false candidates once on any
            // partition, removing them from this list immediately.
            self.candidates.extend([
                Candidate::with_invalid_key(HasControlFlow::Match(source)),
                Candidate::with_invalid_key(HasControlFlow::Loop(source)),
                // Obviously this binding site equals itself, so this checks
                // whether it participates in any equality constraints at all.
                Candidate::with_invalid_key(HasControlFlow::Equal(source, source)),
            ]);
        }

        *old = state;
    }

    /// For the specified set of rules, heuristically choose which control-flow
    /// will minimize redundant work when the generated code is running.
    fn best_control_flow(&mut self, order: &mut [usize]) -> Option<HasControlFlow> {
        // If there are no rules left, none of the candidates will match
        // anything in the `retain_mut` call below, so short-circuit it.
        if order.is_empty() {
            // This is only read in a debug-assert but it's fast so just do it
            self.candidates.clear();
            return None;
        }

        // Remove false candidates, and recompute the candidate sort key for
        // the current set of rules in `order`.
        self.candidates.retain_mut(|candidate| {
            // This binding's state may have changed since we last looked at it.
            let source = match candidate.kind {
                HasControlFlow::Match(source) => source,
                HasControlFlow::Equal(source, _) => source,
                HasControlFlow::Loop(source) => source,
            };
            candidate.state = self.ready[source.index()];

            // Candidates which have already been matched in this partition
            // must not be matched again. (Note that equality constraints are
            // de-duplicated using more complicated equivalence-class checks
            // instead.)
            if candidate.state == BindingState::Matched {
                return false;
            }

            let constrained = candidate
                .kind
                .partition_ignoring_priority(&self.rules, order);

            // Only consider constraints that are present in some rule in the
            // current partition. Note that as we partition the rule set into
            // smaller groups, the number of rules which have a particular
            // kind of constraint can never grow, so a candidate removed here
            // doesn't need to be examined again in this partition.
            if constrained == 0 {
                return false;
            }

            // The sort key is not based solely on how many rules have this
            // constraint, but on how many such rules can go into the same
            // block without violating rule priority. This number can grow
            // as higher-priority rules are removed from the partition, so we
            // _can't_ drop candidates just because this is zero. Since some
            // rule has this constraint, it will become viable in some later
            // partition.
            candidate.count = respect_priority(&self.rules, order, constrained);
            true
        });

        // Set aside candidates which currently can't handle any rules. They
        // can't influence this round, but we'll revisit them later.
        let progress = partition_in_place(&mut self.candidates, |candidate| candidate.count > 0);
        let candidates = &mut self.candidates[..progress];

        // Now separate out the equality constraint candidates, which require
        // special handling, from the normal cases.
        let normals = partition_in_place(candidates, |candidate| {
            !matches!(candidate.kind, HasControlFlow::Equal(_, _))
        });
        let (normals, equals) = candidates.split_at_mut(normals);

        // Find the best normal candidate.
        let mut best = None;
        for candidate in normals.iter() {
            if best.as_ref() < Some(candidate) {
                best = Some(candidate.clone());
            }
        }

        // Everything before this took O(n) time.

        // Equality-constraint candidates are over-approximated. They identify
        // how many rules have some kind of equality constraint involving a
        // particular binding site, but we actually need to find the rules
        // with an equality constraint on a particular _pair_ of binding
        // sites. Those are the intersection of two candidates' rules, so the
        // upper bound on the number of matching rules is whichever candidate
        // is smaller. Do an O(n log n) sort before the O(n^2) all-pairs loop
        // so we can do branch-and-bound style pruning.
        equals.sort_unstable_by(|x, y| y.cmp(x));
        let mut equals = &equals[..];
        while let Some((x, rest)) = equals.split_first() {
            equals = rest;
            if Some(x) < best.as_ref() {
                break;
            }
            if let HasControlFlow::Equal(x_id, _) = x.kind {
                let x_repr = self.equal.find_mut(x_id);
                for y in rest.iter() {
                    if Some(y) < best.as_ref() {
                        break;
                    }
                    if let HasControlFlow::Equal(y_id, _) = y.kind {
                        let y_repr = self.equal.find_mut(y_id);
                        // If x and y both have representatives and they're
                        // the same, skip this pair because we already emitted
                        // this check or a combination of equivalent checks on
                        // this path.
                        if x_repr.is_none() || x_repr != y_repr {
                            // Sort arguments for consistency.
                            let kind = if x_id < y_id {
                                HasControlFlow::Equal(x_id, y_id)
                            } else {
                                HasControlFlow::Equal(y_id, x_id)
                            };
                            let pair = Candidate {
                                kind,
                                count: kind.partition(self.rules, order),
                                // Only treat this as already-emitted if both
                                // bindings are.
                                state: x.state.min(y.state),
                            };
                            if best.as_ref() < Some(&pair) {
                                best = Some(pair);
                            }
                        }
                    }
                }
            }
        }

        best.filter(|candidate| candidate.count > 0)
            .map(|candidate| candidate.kind)
    }
}

/// Places all elements which satisfy the predicate at the beginning of the
/// slice, and all elements which don't at the end. Returns the number of
/// elements in the first partition.
///
/// This function runs in time linear in the number of elements, and calls the
/// predicate exactly once per element. If either partition is empty,
fn partition_in_place<T>(xs: &mut [T], mut pred: impl FnMut(&T) -> bool) -> usize {
    let mut iter = xs.iter_mut();
    let mut partition_point = 0;
    while let Some(a) = iter.next() {
        if pred(a) {
            partition_point += 1;
        } else {
            // `a` belongs in the partition at the end. If there's some later
            // element `b` that belongs in the partition at the beginning,
            // swap them. Working backwards from the end establishes the loop
            // invariant that both ends of the array are partitioned correctly,
            // and only the middle needs to be checked.
            while let Some(b) = iter.next_back() {
                if pred(b) {
                    std::mem::swap(a, b);
                    partition_point += 1;
                    break;
                }
            }
        }
    }
    partition_point
}

fn group_by_mut<T: Eq>(
    mut xs: &mut [T],
    mut pred: impl FnMut(&T, &T) -> bool,
) -> impl Iterator<Item = &mut [T]> {
    std::iter::from_fn(move || {
        if xs.is_empty() {
            None
        } else {
            let mid = xs
                .windows(2)
                .position(|w| !pred(&w[0], &w[1]))
                .map_or(xs.len(), |x| x + 1);
            let slice = std::mem::take(&mut xs);
            let (group, rest) = slice.split_at_mut(mid);
            xs = rest;
            Some(group)
        }
    })
}

#[test]
fn test_group_mut() {
    let slice = &mut [1, 1, 1, 3, 3, 2, 2, 2];
    let mut iter = group_by_mut(slice, |a, b| a == b);
    assert_eq!(iter.next(), Some(&mut [1, 1, 1][..]));
    assert_eq!(iter.next(), Some(&mut [3, 3][..]));
    assert_eq!(iter.next(), Some(&mut [2, 2, 2][..]));
    assert_eq!(iter.next(), None);
}
