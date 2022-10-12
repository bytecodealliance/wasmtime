//! Overlap detection for rules in ISLE.

use rayon::prelude::*;
use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};

use crate::error::{Error, Result, Source, Span};
use crate::lexer::Pos;
use crate::sema::{self, Rule, RuleId, Sym, TermEnv, TermId, TermKind, TypeEnv, VarId};

/// Check for overlap.
pub fn check(tyenv: &TypeEnv, termenv: &TermEnv) -> Result<()> {
    let mut errors = check_overlaps(termenv).report(tyenv, termenv);
    errors.sort_by_key(|err| match err {
        Error::OverlapError { rules, .. } => rules.first().unwrap().1.from,
        _ => Pos::default(),
    });
    match errors.len() {
        0 => Ok(()),
        1 => Err(errors.pop().unwrap()),
        _ => Err(Error::Errors(errors)),
    }
}

/// A graph of rules that overlap in the ISLE source. The edges are undirected.
#[derive(Default)]
struct Errors {
    /// Edges between rules indicating overlap.
    nodes: HashMap<RuleId, HashSet<RuleId>>,
}

impl Errors {
    /// Merge together two Error graphs.
    fn union(mut self, other: Self) -> Self {
        for (id, edges) in other.nodes {
            match self.nodes.entry(id) {
                Entry::Occupied(entry) => entry.into_mut().extend(edges),
                Entry::Vacant(entry) => _ = entry.insert(edges),
            }
        }
        self
    }

    /// Condense the overlap information down into individual errors. We iteratively remove the
    /// nodes from the graph with the highest degree, reporting errors for them and their direct
    /// connections. The goal with reporting errors this way is to prefer reporting rules that
    /// overlap with many others first, and then report other more targeted overlaps later.
    fn report(mut self, tyenv: &TypeEnv, termenv: &TermEnv) -> Vec<Error> {
        let mut errors = Vec::new();

        let get_info = |id: RuleId| {
            let rule = &termenv.rules[id.0];
            let file = rule.pos.file;
            let src = Source::new(
                tyenv.filenames[file].clone(),
                tyenv.file_texts[file].clone(),
            );
            let span = Span::new_single(rule.pos);
            (src, span)
        };

        while let Some((&id, _)) = self
            .nodes
            .iter()
            .max_by_key(|(id, edges)| (edges.len(), *id))
        {
            let node = self.nodes.remove(&id).unwrap();
            for other in node.iter() {
                if let Entry::Occupied(mut entry) = self.nodes.entry(*other) {
                    let back_edges = entry.get_mut();
                    back_edges.remove(&id);
                    if back_edges.is_empty() {
                        entry.remove();
                    }
                }
            }

            // build the real error
            let mut rules = vec![get_info(id)];

            rules.extend(node.into_iter().map(get_info));

            errors.push(Error::OverlapError {
                msg: String::from("rules are overlapping"),
                rules,
            });
        }

        errors
    }

    /// Add a bidirectional edge between two rules in the graph.
    fn add_edge(&mut self, a: RuleId, b: RuleId) {
        // edges are undirected
        self.nodes.entry(a).or_default().insert(b);
        self.nodes.entry(b).or_default().insert(a);
    }
}

/// Determine if any rules overlap in the input that they accept. This checkes every unique pair of
/// rules, as checking rules in aggregate tends to suffer from exponential explosion in the
/// presence of wildcard patterns.
fn check_overlaps(env: &TermEnv) -> Errors {
    struct RulePatterns<'a> {
        rule: &'a Rule,
        pats: Box<[Pattern]>,
    }
    let mut by_term = HashMap::new();
    for rule in env.rules.iter() {
        if let sema::Pattern::Term(_, tid, ref vars) = rule.lhs {
            let is_multi_ctor = match &env.terms[tid.index()].kind {
                &TermKind::Decl { multi, .. } => multi,
                _ => false,
            };
            if is_multi_ctor {
                // Rules for multi-constructors are not checked for
                // overlap: the ctor returns *every* match, not just
                // the first or highest-priority one, so overlap does
                // not actually affect the results.
                continue;
            }

            let mut binds = Vec::new();
            let rule = RulePatterns {
                rule,
                pats: vars
                    .iter()
                    .map(|pat| Pattern::from_sema(env, &mut binds, pat))
                    .collect(),
            };
            by_term.entry(tid).or_insert_with(Vec::new).push(rule);
        }
    }

    // Sequentially identify all rule pairs which are in the same term. We could make this a
    // parallel iterator, but that's harder to read and this loop is fast. Also, Rayon can
    // efficiently partition a vector across multiple CPUs, which it might have more trouble with
    // if this were an iterator.
    let mut pairs = Vec::new();
    for rows in by_term.values() {
        let mut cursor = &rows[..];
        while let Some((row, rest)) = cursor.split_first() {
            cursor = rest;
            pairs.extend(rest.iter().map(|other| (row, other)));
        }
    }

    // Process rule pairs in parallel. Rayon makes this easy and we have independent bite-sized
    // chunks of work, so we might as well take advantage of multiple CPUs if they're available.
    pairs
        .into_par_iter()
        .fold(Errors::default, |mut errs, (left, right)| {
            if left.rule.prio == right.rule.prio {
                if check_overlap_pair(&left.pats, &right.pats) {
                    errs.add_edge(left.rule.id, right.rule.id);
                }
            }
            errs
        })
        .reduce(Errors::default, Errors::union)
}

/// Check if two rules overlap in the inputs they accept.
fn check_overlap_pair(a: &[Pattern], b: &[Pattern]) -> bool {
    debug_assert_eq!(a.len(), b.len());
    let mut worklist: Vec<_> = a.iter().zip(b.iter()).collect();

    while let Some((a, b)) = worklist.pop() {
        // Checking the cross-product of two and-patterns is O(n*m). Merging sorted lists or
        // hash-maps might be faster in practice, but:
        // - The alternatives are not asymptotically faster, because in theory all the subpatterns
        //   might have the same extractor or enum variant, and in that case any approach has to
        //   check all of the cross-product combinations anyway.
        // - It's easier to reason about this doubly-nested loop than about merging sorted lists or
        //   picking the right hash keys.
        // - These lists are always so small that performance doesn't matter.
        for a in a.as_and_subpatterns() {
            for b in b.as_and_subpatterns() {
                let overlap = match (a, b) {
                    (Pattern::Int { value: a }, Pattern::Int { value: b }) => a == b,
                    (Pattern::Const { name: a }, Pattern::Const { name: b }) => a == b,

                    // if it's the same variant or same extractor, check all pairs of subterms
                    (
                        Pattern::Variant {
                            id: a,
                            pats: a_pats,
                        },
                        Pattern::Variant {
                            id: b,
                            pats: b_pats,
                        },
                    )
                    | (
                        Pattern::Extractor {
                            id: a,
                            pats: a_pats,
                        },
                        Pattern::Extractor {
                            id: b,
                            pats: b_pats,
                        },
                    ) if a == b => {
                        debug_assert_eq!(a_pats.len(), b_pats.len());
                        worklist.extend(a_pats.iter().zip(b_pats.iter()));
                        true
                    }

                    // different variants of the same enum definitely do not overlap
                    (Pattern::Variant { .. }, Pattern::Variant { .. }) => false,

                    // an extractor which does not exactly match the other pattern might overlap
                    (Pattern::Extractor { .. }, _) | (_, Pattern::Extractor { .. }) => true,

                    // a wildcard definitely overlaps
                    (Pattern::Wildcard, _) | (_, Pattern::Wildcard) => true,

                    // these patterns can only be paired with patterns of the same type, or
                    // wildcards or extractors, and all those cases are covered above
                    (Pattern::Int { .. } | Pattern::Const { .. } | Pattern::Variant { .. }, _) => {
                        unreachable!()
                    }

                    // and-patterns don't reach here due to as_and_subpatterns
                    (Pattern::And { .. }, _) => unreachable!(),
                };

                if !overlap {
                    return false;
                }
            }
        }
    }
    true
}

/// A version of [`sema::Pattern`] with some simplifications to make overlap checking easier.
#[derive(Debug, Clone)]
enum Pattern {
    /// Integer literal patterns.
    Int {
        value: i128,
    },

    /// Constant literal patterns, such as `$F32`.
    Const {
        name: Sym,
    },

    /// Enum variant constructors.
    Variant {
        id: TermId,
        pats: Box<[Pattern]>,
    },

    /// Conjunctions of patterns.
    And {
        pats: Box<[Pattern]>,
    },

    /// Extractor uses (both fallible and infallible).
    Extractor {
        id: TermId,
        pats: Box<[Pattern]>,
    },

    Wildcard,
}

impl Pattern {
    /// Create a [`Pattern`] from a [`sema::Pattern`]. The major differences between these two
    /// representations are as follows:
    /// 1. Variable bindings are removed and turned into wildcards
    /// 2. Equality constraints are removed and turned into inlined versions of the patterns they
    ///    would have introduced equalities with
    /// 3. [`sema::Pattern::Term`] instances are turned into either [`Pattern::Variant`] or
    ///    [`Pattern::Extractor`] cases depending on their term kind.
    fn from_sema(env: &TermEnv, binds: &mut Vec<(VarId, Pattern)>, pat: &sema::Pattern) -> Self {
        match pat {
            sema::Pattern::BindPattern(_, id, pat) => {
                let pat = Self::from_sema(env, binds, pat);
                binds.push((*id, pat.clone()));
                pat
            }

            sema::Pattern::Var(_, id) => {
                for (vid, pat) in binds.iter().rev() {
                    if vid == id {
                        // We inline equality constraints for two reasons: we specialize on the
                        // spine of related patterns only, so more specific information about
                        // individual values isn't necessarily helpful; we consider overlap
                        // checking to be an over-approximation of overlapping rules, so handling
                        // equalities ends up being best-effort. As an approximation, we use
                        // whatever pattern happened to be at the binding of the variable for all
                        // of the cases where it's used for equality. For example, in the following
                        // rule:
                        //
                        // > (rule (example x @ (Enum.Variant y) x) ...)
                        //
                        // we will only specialize up to `(Enum.Variant _)`, so any more specific
                        // runtime values of `y` won't end up helping to identify overlap. As a
                        // result, we rewrite the patterns in the rule to look more like the
                        // following, as it greatly simplifies overlap checking.
                        //
                        // > (rule (example (Enum.Variant _) (Enum.Variant _)) ...)
                        //
                        // Cases that this scheme won't handle look like the following:
                        //
                        // > (rule (example2 2 3) ...)
                        // > (rule (example2 x x) ...)
                        //
                        // As in this case we'll not make use of the information that `2` and `3`
                        // aren't equal to know that the rules don't overlap. One approach that we
                        // could take here is delaying substitution to the point where a variable
                        // binding has been specialized, turning the rules into the following once
                        // specialization had occurred for `2`:
                        //
                        // > (rule (example2 2 3) ...)
                        // > (rule (example2 2 2) ...)
                        return pat.clone();
                    }
                }

                binds.push((*id, Pattern::Wildcard));
                Pattern::Wildcard
            }

            sema::Pattern::ConstInt(_, value) => Pattern::Int { value: *value },
            sema::Pattern::ConstPrim(_, name) => Pattern::Const { name: *name },

            &sema::Pattern::Term(_, id, ref pats) => {
                let pats = pats
                    .iter()
                    .map(|pat| Pattern::from_sema(env, binds, pat))
                    .collect();

                match &env.terms[id.0].kind {
                    TermKind::EnumVariant { .. } => Pattern::Variant { id, pats },
                    TermKind::Decl { .. } => Pattern::Extractor { id, pats },
                }
            }

            sema::Pattern::Wildcard(_) => Pattern::Wildcard,

            sema::Pattern::And(_, pats) => {
                let pats = pats
                    .iter()
                    .map(|pat| Pattern::from_sema(env, binds, pat))
                    .collect();
                Pattern::And { pats }
            }
        }
    }

    /// If this is an and-pattern, return its subpatterns. Otherwise pretend like there's an
    /// and-pattern which has this as its only subpattern, and return self as a single-element
    /// slice.
    fn as_and_subpatterns(&self) -> &[Pattern] {
        if let Pattern::And { pats } = self {
            pats
        } else {
            std::slice::from_ref(self)
        }
    }
}
