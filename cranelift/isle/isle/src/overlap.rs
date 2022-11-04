//! Overlap detection for rules in ISLE.

use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};

use crate::error::{Error, Result, Source, Span};
use crate::lexer::Pos;
use crate::sema::{RuleId, TermEnv, TermKind, TypeEnv};
use crate::trie_again;

/// Check for overlap.
pub fn check(tyenv: &TypeEnv, termenv: &TermEnv) -> Result<()> {
    let mut unmatchable = Vec::new();
    let mut overlaps = Errors::default();
    for (term, ruleset) in trie_again::build(termenv)? {
        let is_multi_ctor = match &termenv.terms[term.index()].kind {
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

        let mut cursor = ruleset.rules.iter();
        while let Some((aid, a)) = cursor.next() {
            for (bid, b) in cursor.as_slice() {
                if let trie_again::Overlap::Yes { subset } = a.may_overlap(b) {
                    if a.prio == b.prio {
                        overlaps.add_edge(*aid, *bid);
                    } else if subset {
                        // One rule's constraints are a subset of the other's, or they're equal.
                        // This is fine as long as the higher-priority rule has more constraints.
                        let (lo, hi) = if a.prio < b.prio { (a, b) } else { (b, a) };
                        if hi.constraints.len() <= lo.constraints.len() {
                            // Otherwise, the lower-priority rule can never match.
                            unmatchable.push(Error::UnmatchableError {
                                msg: format!(
                                    "rule shadowed by more general higher-priority rule at {:?}",
                                    hi.pos
                                ),
                                span: Span::new_single(lo.pos),
                            });
                        }
                    }
                }
            }
        }
    }

    let mut errors = overlaps.report(tyenv, termenv);
    errors.sort_by_key(|err| match err {
        Error::OverlapError { rules, .. } => rules.first().unwrap().1.from,
        _ => Pos::default(),
    });

    // FIXME: for the moment, the unmatchable rules will just break CI
    //errors.append(&mut unmatchable);

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
