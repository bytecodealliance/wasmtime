//! Put "sea of nodes" representation of a `RuleSet` into a sequential order.
#![allow(missing_docs)]

use std::cmp::Reverse;

use crate::lexer::Pos;
use crate::trie_again::{Binding, BindingId, Constraint, Overlap, RuleSet};

#[derive(Default)]
pub struct Block {
    pub cases: Vec<Case>,
}

pub struct Case {
    /// Before evaluating this case, emit let-bindings in this order.
    pub bind_order: Vec<BindingId>,
    pub check: Condition,
}

pub enum Condition {
    Match {
        source: BindingId,
        arms: Vec<MatchArm>,
    },
    Equal {
        a: BindingId,
        b: BindingId,
        body: Block,
    },
    Loop {
        result: BindingId,
        body: Block,
    },
    Result {
        pos: Pos,
        result: BindingId,
    },
}

pub struct MatchArm {
    pub constraint: Constraint,
    pub bindings: Vec<Option<BindingId>>,
    pub body: Block,
}

pub fn serialize(rules: &RuleSet) -> Block {
    let mut order = Vec::from_iter(0..rules.rules.len());
    Ready::new(rules).sort(&mut order)
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum MatchKind {
    // Sort concrete constraints before equality constraints so pattern matches cluster together.
    Constraint(BindingId),
    Equal(BindingId, BindingId),
    // Sort multi-terms after everything else so loops are nested as deeply as possible.
    Iterator(BindingId),
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum Readiness {
    Unavailable,
    Available,
    Emitted,
    Matched,
}

#[derive(Clone, Debug)]
struct Candidate {
    count: usize,
    state: Readiness,
    kind: MatchKind,
}

impl Candidate {
    fn key(&self) -> impl Ord {
        // We prefer to match as many rules at once as possible.
        // Break ties by preferring bindings we've already emitted.
        (self.count, self.state)
    }
}

struct Ready<'a> {
    rules: &'a RuleSet,
    ready: Vec<Readiness>,
    candidates: Vec<Candidate>,
    bind_order: Vec<BindingId>,
    block: Block,
}

impl<'a> Ready<'a> {
    fn new(rules: &'a RuleSet) -> Ready<'a> {
        let mut result = Ready {
            rules,
            ready: vec![Readiness::Unavailable; rules.bindings.len()],
            candidates: Default::default(),
            bind_order: Default::default(),
            block: Default::default(),
        };
        result.add_bindings();
        result
    }

    fn new_block(&mut self) -> Ready {
        Ready {
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
            if self.ready(idx) < Readiness::Available {
                if binding
                    .sources()
                    .iter()
                    .all(|&source| self.ready(source) >= Readiness::Available)
                {
                    self.set_ready(idx, Readiness::Available);
                }
            }
        }
    }

    fn use_expr(&mut self, name: BindingId) {
        if self.ready(name) < Readiness::Emitted {
            self.set_ready(name, Readiness::Emitted);
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
                // Only let-bind variant constructors if they have some fields. Building a
                // variant with no fields is cheap, but don't duplicate more complex
                // expressions.
                Binding::MakeVariant { fields, .. } => !fields.is_empty(),
                _ => true,
            };
            if let_bind {
                self.bind_order.push(name);
            }
        }
    }

    fn ready(&self, source: BindingId) -> Readiness {
        self.ready[source.index()]
    }

    fn set_ready(&mut self, source: BindingId, state: Readiness) {
        let old = &mut self.ready[source.index()];
        debug_assert!(*old <= state);

        // Add candidates for this binding, but only when it first becomes available.
        if let Readiness::Unavailable = old {
            self.candidates.extend(
                // A binding site will have at most one of these kinds of constraint, and many have
                // none. But `best_single_binding` has to check all candidates anyway, so let it
                // figure out which (if any) of these are applicable. It will only check false
                // candidates once on any partition, removing them from this list immediately.
                [
                    MatchKind::Constraint(source),
                    MatchKind::Iterator(source),
                    // Obviously this binding site equals itself, so this checks whether it
                    // participates in any equality constraints at all.
                    MatchKind::Equal(source, source),
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
        while !order.is_empty() {
            if let Some(best) = self.best_single_binding(order) {
                let partition_point = split_on(&self.rules, order, best);
                debug_assert!(partition_point > 0);
                let (this, rest) = order.split_at_mut(partition_point);
                order = rest;

                let check = match best {
                    MatchKind::Constraint(source) => {
                        self.use_expr(source);
                        self.add_bindings();
                        let mut arms = Vec::new();

                        let get_constraint =
                            |idx: usize| self.rules.rules[idx].get_constraint(source).unwrap();
                        this.sort_unstable_by_key(|&idx| get_constraint(idx));
                        for g in group_by_mut(this, |&a, &b| get_constraint(a) == get_constraint(b))
                        {
                            let mut child = self.new_block();
                            // Applying a constraint moves the discriminant from Emitted to Matched, but
                            // only within the constraint's match arm; later fallthrough cases may need to
                            // match this discriminant again.
                            child.set_ready(source, Readiness::Matched);

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
                                    // Matching a pattern makes its bindings available, and also emits code to bind them.
                                    child.set_ready(binding, Readiness::Emitted);
                                    changed = true;
                                }
                            }

                            // As an optimization, only propagate availability if we changed any binding's readiness.
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

                        Condition::Match { source, arms }
                    }

                    MatchKind::Equal(a, b) => {
                        self.use_expr(a);
                        self.use_expr(b);
                        self.add_bindings();

                        let mut child = self.new_block();
                        child.set_ready(a, Readiness::Matched);
                        child.set_ready(b, Readiness::Matched);
                        let body = child.sort(this);

                        Condition::Equal { a, b, body }
                    }

                    MatchKind::Iterator(source) => {
                        let result = self
                            .rules
                            .find_binding(&Binding::Iterator { source })
                            .unwrap();
                        self.use_expr(source);
                        self.add_bindings();

                        let mut child = self.new_block();
                        child.set_ready(source, Readiness::Matched);
                        child.set_ready(result, Readiness::Emitted);
                        child.add_bindings();
                        let body = child.sort(this);

                        Condition::Loop { result, body }
                    }
                };

                self.finish_case(check);
            } else {
                debug_assert_eq!(
                    self.candidates
                        .iter()
                        .filter(|c| c.state != Readiness::Matched)
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
                    self.finish_case(Condition::Result {
                        pos: rule.pos,
                        result: rule.result,
                    });
                }
                break;
            }
        }

        self.block
    }

    fn finish_case(&mut self, check: Condition) {
        let bind_order = std::mem::take(&mut self.bind_order);
        self.block.cases.push(Case { bind_order, check });
    }

    fn best_single_binding(&mut self, order: &mut [usize]) -> Option<MatchKind> {
        // Remove false candidates, and recompute candidate state for the current set of rules in
        // `order`. Note that as we partition the rule set into smaller groups, the number of rules
        // which have a particular kind of constraint can never grow, so a candidate removed here
        // doesn't need to be examined again in this partition.
        self.candidates.retain_mut(|candidate| {
            // This binding's state may have changed since we last looked at it.
            let source = match candidate.kind {
                MatchKind::Constraint(source) => source,
                MatchKind::Equal(source, _) => source,
                MatchKind::Iterator(source) => source,
            };
            candidate.state = self.ready[source.index()];

            // Never evaluate concrete constraints on binding sites that we already matched. Either
            // we matched against a concrete constraint, in which case we shouldn't do it again; or
            // we matched an equality constraint, in which case we know there are no concrete
            // constraints on the same binding sites in these rules.
            if let Candidate {
                state: Readiness::Matched,
                kind: MatchKind::Constraint(_),
                ..
            } = candidate
            {
                return false;
            }

            // Only consider constraints that are present in some rule in the current partition.
            let constrained = find_constraints(&self.rules, order, candidate.kind);
            if constrained == 0 {
                return false;
            }

            // The sort key below is not based solely on how many rules have this constraint, but
            // on how many such rules can go into the same block without violating rule priority.
            candidate.count = respect_priority(&self.rules, order, constrained);

            // Even if there are no satisfying rules now, we still need to keep this candidate.
            // Since some rule has this constraint, it will become viable in some later partition.
            true
        });

        self.candidates.sort_unstable_by_key(|candidate| {
            (
                // Put unmatched binding sites first, for partition_point below.
                candidate.state == Readiness::Matched,
                Reverse(candidate.key()),
                // Final tie-breaker: prefer constraints on the earliest binding site.
                candidate.kind,
            )
        });
        let unmatched = self
            .candidates
            .partition_point(|candidate| candidate.state != Readiness::Matched);
        let (mut unmatched, matched) = self.candidates.split_at_mut(unmatched);

        let mut best = Candidate {
            count: 0,
            // All valid candidates have count > 0 so the other fields don't matter.
            state: Readiness::Unavailable,
            kind: MatchKind::Constraint(0.try_into().unwrap()),
        };
        while let Some((candidate, rest)) = unmatched.split_first_mut() {
            unmatched = rest;
            if candidate.key() <= best.key() {
                break;
            }
            match candidate.kind {
                MatchKind::Equal(a, _) => {
                    for other in unmatched.iter().chain(matched.iter()) {
                        if let MatchKind::Equal(b, _) = other.kind {
                            let mut new = Candidate {
                                // `split_on` will find the intersection of these two candidates'
                                // rules, and the best case is one is a subset of the other.
                                count: candidate.count.min(other.count),
                                // Only treat this as already-emitted if both bindings are.
                                state: candidate.state.min(other.state),
                                // Sort arguments for consistency.
                                kind: if a < b {
                                    MatchKind::Equal(a, b)
                                } else {
                                    MatchKind::Equal(b, a)
                                },
                            };
                            if new.key() > best.key() {
                                new.count = split_on(&self.rules, order, new.kind);
                                if new.key() > best.key() {
                                    best = new;
                                    // `new` can be no better than either `candidate` or `other`,
                                    // but if it's no worse, then it's better than all our other
                                    // candidates.
                                    if best.key() == candidate.key() {
                                        return Some(best.kind);
                                    }
                                }
                            }
                        }
                    }
                }
                MatchKind::Constraint(_) | MatchKind::Iterator(_) => return Some(candidate.kind),
            }
        }
        if best.count > 0 {
            Some(best.kind)
        } else {
            None
        }
    }
}

fn split_on(rules: &RuleSet, order: &mut [usize], kind: MatchKind) -> usize {
    let partition_point = find_constraints(rules, order, kind);
    respect_priority(rules, order, partition_point)
}

/// Put rules which constrain this binding site first and return the number of rules found.
fn find_constraints(rules: &RuleSet, order: &mut [usize], kind: MatchKind) -> usize {
    let partition_point = partition_in_place(order, |&idx| {
        let rule = &rules.rules[idx];
        match kind {
            MatchKind::Equal(x, y) => rule
                .equals
                .find(x)
                .zip(rule.equals.find(y))
                .filter(|(x, y)| x == y)
                .is_some(),
            MatchKind::Constraint(binding_id) => rule.get_constraint(binding_id).is_some(),
            MatchKind::Iterator(binding_id) => rule.iterators.contains(&binding_id),
        }
    });
    partition_point
}

/// If we're going to match on just one binding site, then we have to leave rules which don't
/// constrain that binding site for a "fall-through" case. But that means we have to ensure
/// that, for any possible input, we _will_ fall through if one of those unconstrained rules
/// is the highest-priority rule matching that input. That works if either the unconstrained
/// rule is lower priority, or the lower-priority rules can never match on the same inputs as
/// the unconstrained rules.
fn respect_priority(rules: &RuleSet, order: &mut [usize], partition_point: usize) -> usize {
    let (constrained, unconstrained) = order.split_at_mut(partition_point);
    if unconstrained.is_empty() {
        return partition_point;
    }

    partition_in_place(constrained, |&idx| {
        let rule = &rules.rules[idx];
        unconstrained.iter().all(|&idx| {
            let other = &rules.rules[idx];
            // If two rules have the same priority, we can assume they don't overlap since
            // otherwise overlap checking would have already rejected this rule set.
            rule.prio >= other.prio || rule.may_overlap(other) == Overlap::No
        })
    })
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
