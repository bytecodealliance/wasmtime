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
use crate::trie_again::{Binding, BindingId, Constraint, Overlap, RuleSet};

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
    /// first in `order`, and return the number of rules found. No ordering is
    /// guaranteed within either partition, which is fine because later we'll
    /// recursively sort both partitions.
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

#[derive(Clone, Debug)]
struct Candidate {
    count: usize,
    state: BindingState,
    kind: HasControlFlow,
}

impl Candidate {
    fn key(&self) -> impl Ord {
        // We prefer to match as many rules at once as possible. Break ties by
        // preferring bindings we've already emitted.
        (self.count, self.state)
    }
}

struct Decomposition<'a> {
    rules: &'a RuleSet,
    ready: Vec<BindingState>,
    candidates: Vec<Candidate>,
    bind_order: Vec<BindingId>,
    block: Block,
}

impl<'a> Decomposition<'a> {
    fn new(rules: &'a RuleSet) -> Decomposition<'a> {
        let mut result = Decomposition {
            rules,
            ready: vec![BindingState::Unavailable; rules.bindings.len()],
            candidates: Default::default(),
            bind_order: Default::default(),
            block: Default::default(),
        };
        result.add_bindings();
        result
    }

    fn new_block(&mut self) -> Decomposition {
        Decomposition {
            rules: self.rules,
            ready: self.ready.clone(),
            candidates: self.candidates.clone(),
            bind_order: Default::default(),
            block: Default::default(),
        }
    }

    fn add_bindings(&mut self) {
        for (idx, binding) in self.rules.bindings.iter().enumerate() {
            // We only add these bindings when matching a corresponding constraint.
            if matches!(
                binding,
                Binding::Iterator { .. } | Binding::MatchVariant { .. } | Binding::MatchSome { .. }
            ) {
                continue;
            }

            // TODO: proactively put some bindings in `Emitted` state
            // That makes them visible to the best-binding heuristic, which prefers to match on
            // already-emitted bindings first. This helps to sort cheap computations before
            // expensive ones.

            let idx = idx.try_into().unwrap();
            if self.ready(idx) < BindingState::Available {
                if binding
                    .sources()
                    .iter()
                    .all(|&source| self.ready(source) >= BindingState::Available)
                {
                    self.set_ready(idx, BindingState::Available);
                }
            }
        }
    }

    fn use_expr(&mut self, name: BindingId) {
        if self.ready(name) < BindingState::Emitted {
            self.set_ready(name, BindingState::Emitted);
            let binding = &self.rules.bindings[name.index()];
            for &source in binding.sources() {
                self.use_expr(source);
            }
            let let_bind = match binding {
                // Never let-bind trivial expressions.
                Binding::MatchTuple { .. } => false,
                Binding::ConstInt { .. } => false,
                Binding::ConstPrim { .. } => false,
                Binding::Argument { .. } => false,
                // Only let-bind variant constructors if they have some fields.
                // Building a variant with no fields is cheap, but don't
                // duplicate more complex expressions.
                Binding::MakeVariant { fields, .. } => !fields.is_empty(),
                _ => true,
            };
            if let_bind {
                self.bind_order.push(name);
            }
        }
    }

    fn ready(&self, source: BindingId) -> BindingState {
        self.ready[source.index()]
    }

    fn set_ready(&mut self, source: BindingId, state: BindingState) {
        let old = &mut self.ready[source.index()];
        debug_assert!(*old <= state);

        // Add candidates for this binding, but only when it first becomes
        // available.
        if let BindingState::Unavailable = old {
            self.candidates.extend(
                // A binding site will have at most one of these kinds of
                // constraint, and many have none. But `best_single_binding`
                // has to check all candidates anyway, so let it figure out
                // which (if any) of these are applicable. It will only check
                // false candidates once on any partition, removing them from
                // this list immediately.
                [
                    HasControlFlow::Match(source),
                    HasControlFlow::Loop(source),
                    // Obviously this binding site equals itself, so this
                    // checks whether it participates in any equality
                    // constraints at all.
                    HasControlFlow::Equal(source, source),
                ]
                .into_iter()
                .map(|kind| Candidate {
                    // count will be filled in later by `best_single_binding`
                    count: 0,
                    state,
                    kind,
                }),
            );
        }

        *old = state;
    }

    fn sort(mut self, mut order: &mut [usize]) -> Block {
        while let Some(best) = self.best_single_binding(order) {
            let partition_point = best.partition(&self.rules, order);
            debug_assert!(partition_point > 0);
            let (this, rest) = order.split_at_mut(partition_point);
            order = rest;

            let control_flow = self.make_control_flow(best, this);
            self.add_step(control_flow);
        }

        debug_assert_eq!(
            self.candidates
                .iter()
                .filter(|c| c.state != BindingState::Matched)
                .count(),
            0
        );
        order.sort_unstable_by_key(|&idx| (Reverse(self.rules.rules[idx].prio), idx));
        for &idx in order.iter() {
            let rule = &self.rules.rules[idx];
            for &impure in rule.impure.iter() {
                self.use_expr(impure);
            }
            self.use_expr(rule.result);
            self.add_step(ControlFlow::Return {
                pos: rule.pos,
                result: rule.result,
            });
        }

        self.block
    }

    fn make_control_flow(&mut self, best: HasControlFlow, order: &mut [usize]) -> ControlFlow {
        match best {
            HasControlFlow::Match(source) => {
                self.use_expr(source);
                self.add_bindings();
                let mut arms = Vec::new();

                let get_constraint =
                    |idx: usize| self.rules.rules[idx].get_constraint(source).unwrap();
                order.sort_unstable_by_key(|&idx| get_constraint(idx));
                for g in group_by_mut(order, |&a, &b| get_constraint(a) == get_constraint(b)) {
                    let mut child = self.new_block();
                    // Applying a constraint moves the discriminant
                    // from Emitted to Matched, but only within the
                    // constraint's match arm; later fallthrough cases
                    // may need to match this discriminant again.
                    child.set_ready(source, BindingState::Matched);

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
                self.use_expr(a);
                self.use_expr(b);
                self.add_bindings();

                let mut child = self.new_block();
                child.set_ready(a, BindingState::Matched);
                child.set_ready(b, BindingState::Matched);
                let body = child.sort(order);

                ControlFlow::Equal { a, b, body }
            }

            HasControlFlow::Loop(source) => {
                let result = self
                    .rules
                    .find_binding(&Binding::Iterator { source })
                    .unwrap();
                self.use_expr(source);
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

    fn add_step(&mut self, check: ControlFlow) {
        let bind_order = std::mem::take(&mut self.bind_order);
        self.block.steps.push(EvalStep { bind_order, check });
    }

    fn best_single_binding(&mut self, order: &mut [usize]) -> Option<HasControlFlow> {
        // If there are no rules left, none of the candidates will match
        // anything in the `retain_mut` call below, so short-circuit it.
        if order.is_empty() {
            // This is only read in a debug-assert but it's fast so just do it
            self.candidates.clear();
            return None;
        }

        // Remove false candidates, and recompute candidate state for the
        // current set of rules in `order`. Note that as we partition the rule
        // set into smaller groups, the number of rules which have a particular
        // kind of constraint can never grow, so a candidate removed here
        // doesn't need to be examined again in this partition.
        self.candidates.retain_mut(|candidate| {
            // This binding's state may have changed since we last looked at it.
            let source = match candidate.kind {
                HasControlFlow::Match(source) => source,
                HasControlFlow::Equal(source, _) => source,
                HasControlFlow::Loop(source) => source,
            };
            candidate.state = self.ready[source.index()];

            // Never evaluate concrete constraints on binding sites that
            // we already matched. Either we matched against a concrete
            // constraint, in which case we shouldn't do it again; or we
            // matched an equality constraint, in which case we know there
            // are no concrete constraints on the same binding sites in these
            // rules.
            // FIXME: discard matched loops too; only matched equals are still useful
            if let Candidate {
                state: BindingState::Matched,
                kind: HasControlFlow::Match(_),
                ..
            } = candidate
            {
                return false;
            }

            // Only consider constraints that are present in some rule in the
            // current partition.
            let constrained = candidate
                .kind
                .partition_ignoring_priority(&self.rules, order);
            if constrained == 0 {
                return false;
            }

            // The sort key below is not based solely on how many rules have
            // this constraint, but on how many such rules can go into the same
            // block without violating rule priority.
            candidate.count = respect_priority(&self.rules, order, constrained);

            // Even if there are no satisfying rules now, we still need to keep
            // this candidate. Since some rule has this constraint, it will
            // become viable in some later partition.
            true
        });

        self.candidates.sort_unstable_by_key(|candidate| {
            (
                // Put unmatched binding sites first, for partition_point
                // below.
                candidate.state == BindingState::Matched,
                Reverse(candidate.key()),
                // Final tie-breaker: prefer constraints on the earliest
                // binding site.
                candidate.kind,
            )
        });
        let unmatched = self
            .candidates
            .partition_point(|candidate| candidate.state != BindingState::Matched);
        let (mut unmatched, matched) = self.candidates.split_at_mut(unmatched);

        let mut best = Candidate {
            count: 0,
            // All valid candidates have count > 0 so the other fields don't
            // matter.
            state: BindingState::Unavailable,
            kind: HasControlFlow::Match(0.try_into().unwrap()),
        };
        while let Some((candidate, rest)) = unmatched.split_first_mut() {
            unmatched = rest;
            if candidate.key() <= best.key() {
                break;
            }
            match candidate.kind {
                HasControlFlow::Equal(a, _) => {
                    for other in unmatched.iter().chain(matched.iter()) {
                        if let HasControlFlow::Equal(b, _) = other.kind {
                            let mut new = Candidate {
                                // `split_on` will find the intersection of
                                // these two candidates' rules, and the best
                                // case is one is a subset of the other.
                                count: candidate.count.min(other.count),
                                // Only treat this as already-emitted if both
                                // bindings are.
                                state: candidate.state.min(other.state),
                                // Sort arguments for consistency.
                                kind: if a < b {
                                    HasControlFlow::Equal(a, b)
                                } else {
                                    HasControlFlow::Equal(b, a)
                                },
                            };
                            if new.key() > best.key() {
                                new.count = new.kind.partition(&self.rules, order);
                                if new.key() > best.key() {
                                    best = new;
                                    // `new` can be no better than either
                                    // `candidate` or `other`, but if it's no
                                    // worse, then it's better than all our
                                    // other candidates.
                                    if best.key() == candidate.key() {
                                        return Some(best.kind);
                                    }
                                }
                            }
                        }
                    }
                }
                HasControlFlow::Match(_) | HasControlFlow::Loop(_) => return Some(candidate.kind),
            }
        }
        if best.count > 0 {
            Some(best.kind)
        } else {
            None
        }
    }
}

fn partition_in_place<T>(xs: &mut [T], mut f: impl FnMut(&T) -> bool) -> usize {
    let mut iter = xs.iter_mut();
    let mut partition_point = 0;
    while let Some(a) = iter.next() {
        if f(a) {
            partition_point += 1;
        } else if let Some(b) = iter.rfind(|b| f(b)) {
            std::mem::swap(a, b);
            partition_point += 1;
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
