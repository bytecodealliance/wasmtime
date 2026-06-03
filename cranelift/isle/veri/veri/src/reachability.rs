use std::collections::{HashMap, HashSet};

use cranelift_isle::{
    sema::TermId,
    trie_again::{Binding, RuleSet},
};

pub struct Reachability {
    reachable: HashMap<TermId, HashSet<TermId>>,
}

impl Reachability {
    pub fn build(term_rule_sets: &HashMap<TermId, RuleSet>) -> Self {
        let mut reachable = HashMap::new();
        for term_id in term_rule_sets.keys() {
            reachable.insert(*term_id, search(*term_id, term_rule_sets));
        }
        Self { reachable }
    }

    /// Set of terms reachable from the the given source.
    pub fn reachable(&self, source: TermId) -> &HashSet<TermId> {
        &self.reachable[&source]
    }

    /// Report whether the term is included in a cycle.
    pub fn is_cyclic(&self, term_id: TermId) -> bool {
        self.reachable(term_id).contains(&term_id)
    }
}

/// Search for all terms reachable from the source.
fn search(source: TermId, term_rule_sets: &HashMap<TermId, RuleSet>) -> HashSet<TermId> {
    let mut reachable = HashSet::new();
    let mut stack = vec![source];

    while let Some(term_id) = stack.pop() {
        if !term_rule_sets.contains_key(&term_id) {
            continue;
        }

        let used = used_terms(&term_rule_sets[&term_id]);
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

pub fn used_terms(rule_set: &RuleSet) -> HashSet<TermId> {
    rule_set
        .bindings
        .iter()
        .filter_map(binding_used_term)
        .collect()
}

pub fn binding_used_term(binding: &Binding) -> Option<TermId> {
    match binding {
        Binding::Constructor { term, .. } | Binding::Extractor { term, .. } => Some(*term),
        // TODO(mbm): make variant uses the variant constructor term?
        _ => None,
    }
}
