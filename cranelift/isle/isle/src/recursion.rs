//! Recursion checking for ISLE terms.

use std::collections::{HashMap, HashSet};

use crate::{
    error::{Error, Span},
    sema::{TermEnv, TermId},
    trie_again::{Binding, RuleSet},
};

/// Check for recursive terms.
pub fn check(terms: &[(TermId, RuleSet)], termenv: &TermEnv) -> Result<(), Vec<Error>> {
    let term_rule_sets: HashMap<TermId, &RuleSet> = terms
        .iter()
        .map(|(term_id, rule_set)| (*term_id, rule_set))
        .collect();

    let mut errors = Vec::new();
    for (term_id, _) in terms {
        // Check if this term is involved in a reference cycle.
        let reachable = terms_reachable_from(*term_id, &term_rule_sets);
        let is_cyclic = reachable.contains(term_id);

        // Lookup if this term is explicitly marked recursive.
        let term = &termenv.terms[term_id.index()];
        let is_marked_recursive = term.is_recursive();

        // Require the two to agree.
        match (is_cyclic, is_marked_recursive) {
            (true, true) | (false, false) => {}
            (true, false) => {
                errors.push(Error::RecursionError {
                    msg: "Term is recursive but does not have the `rec` attribute".to_string(),
                    span: Span::new_single(term.decl_pos),
                });
            }
            (false, true) => {
                errors.push(Error::RecursionError {
                    msg: "Term has the `rec` attribute but is not recursive".to_string(),
                    span: Span::new_single(term.decl_pos),
                });
            }
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Search for all terms reachable from the source.
fn terms_reachable_from(
    source: TermId,
    term_rule_sets: &HashMap<TermId, &RuleSet>,
) -> HashSet<TermId> {
    let mut reachable = HashSet::new();
    let mut stack = vec![source];

    while let Some(term_id) = stack.pop() {
        if !term_rule_sets.contains_key(&term_id) {
            continue;
        }

        let used = terms_in_rule_set(&term_rule_sets[&term_id]);
        for used_term_id in used {
            if reachable.contains(&used_term_id) {
                continue;
            }
            reachable.insert(used_term_id);
            stack.push(used_term_id);
        }
    }

    reachable
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
