//! Recursion checking for ISLE terms.

use std::collections::{HashMap, HashSet};

use crate::{
    error::{Error, Span},
    sema::{TermEnv, TermId},
    trie_again::{Binding, RuleSet},
};

/// Check for recursive terms.
pub fn check(terms: &[(TermId, RuleSet)], termenv: &TermEnv) -> Result<(), Vec<Error>> {
    // Search for cycles in the term dependency graph.
    let cyclic_terms = terms_in_cycles(terms);

    // Cyclic terms should be explicitly permitted with the `rec` attribute.
    let mut errors = Vec::new();
    for term_id in cyclic_terms {
        // Error if term is not explicitly marked recursive.
        let term = &termenv.terms[term_id.index()];
        if !term.is_recursive() {
            errors.push(Error::RecursionError {
                msg: "Term is recursive but does not have the `rec` attribute".to_string(),
                span: Span::new_single(term.decl_pos),
            });
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

// Find terms that are in cycles in the term dependency graph.
fn terms_in_cycles(terms: &[(TermId, RuleSet)]) -> HashSet<TermId> {
    // Construct term dependency graph.
    let edges: HashMap<TermId, HashSet<TermId>> = terms
        .iter()
        .map(|(term_id, rule_set)| (*term_id, terms_in_rule_set(rule_set)))
        .collect();

    // Depth-first search with a stack.
    enum Event {
        Enter(TermId),
        Exit(TermId),
    }
    let mut stack = Vec::from_iter(edges.keys().copied().map(Event::Enter));

    // State of each term.
    enum State {
        Visiting,
        Visited,
    }
    let mut states = HashMap::new();

    // Maintain current path.
    let mut path = Vec::new();

    // Collect terms that are in cycles.
    let mut in_cycle = HashSet::new();

    // Process DFS stack.
    while let Some(event) = stack.pop() {
        match event {
            Event::Enter(term_id) => match states.get(&term_id) {
                None => {
                    states.insert(term_id, State::Visiting);
                    path.push(term_id);
                    stack.push(Event::Exit(term_id));
                    if let Some(deps) = edges.get(&term_id) {
                        for dep in deps {
                            stack.push(Event::Enter(*dep));
                        }
                    }
                }
                Some(State::Visiting) => {
                    // Cycle detected. Reconstruct the cycle from path.
                    let begin = path
                        .iter()
                        .rposition(|&t| t == term_id)
                        .expect("cycle origin should be in path");
                    in_cycle.extend(&path[begin..]);
                }
                Some(State::Visited) => {}
            },
            Event::Exit(term_id) => {
                states.insert(term_id, State::Visited);
                let last = path.pop().expect("exit with empty path");
                debug_assert_eq!(last, term_id, "exit term does not match last path term");
            }
        }
    }

    debug_assert!(path.is_empty(), "search finished with non-empty path");

    in_cycle
}

fn terms_in_rule_set(rule_set: &RuleSet) -> HashSet<TermId> {
    rule_set
        .bindings
        .iter()
        .filter_map(binding_used_term)
        .collect()
}

fn binding_used_term(binding: &Binding) -> Option<TermId> {
    match binding {
        Binding::Constructor { term, .. } | Binding::Extractor { term, .. } => Some(*term),
        _ => None,
    }
}
