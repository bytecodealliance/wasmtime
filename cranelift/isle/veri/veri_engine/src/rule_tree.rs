use std::collections::HashMap;

use crate::solver::run_solver;
use crate::type_inference::Solution;
use crate::{interp::Context, termname::pattern_contains_termname};
use cranelift_isle as isle;
use isle::sema::{Pattern, Rule, RuleId, TermEnv, TypeEnv};
use itertools::Itertools;
use veri_annotation::parser_wrapper::AnnotationEnv;
use veri_ir::{
    all_query_widths, BoundVar, RulePath, RuleSemantics, RuleTree, Type, UndefinedTerm,
    VerificationResult,
};

/// Recursively build a rule tree of possible rewrites, connected by undefined
/// terms on the left hand sides (LHS) and right hand sides (RHS).
pub fn build_rule_tree_rec(
    ctx: &mut Context<'_>,
    rule: &Rule,
    termenv: &TermEnv,
    typeenv: &TypeEnv,
    depth: usize,
    max_depth: usize,
) -> RuleTree {
    // Basic sanity check to avoid infinite cycles, might want to add actual
    // cycle checking later.
    assert!(depth <= max_depth, "Exceeded maximum rule tree depth!");

    // Get the semantics for this specific rule
    let sol = &ctx.typesols[&rule.id];
    let rule_sem = RuleSemantics {
        lhs: sol.lhs.clone(),
        rhs: sol.rhs.clone(),
        assumptions: sol.assumptions.clone(),
        quantified_vars: sol.quantified_vars.clone(),
        free_vars: sol.free_vars.clone(),
        tyctx: sol.tyctx.to_owned(),
        lhs_undefined_terms: vec![],
        rhs_undefined_terms: vec![],
    };

    // If we are at the root of the tree, we should not have any undefined
    // terms on the left hand side
    if depth == 0 {
        assert!(
            rule_sem.lhs_undefined_terms.is_empty(),
            "Cannot have undefined terms in the LHS of the initial root rule, found: {:?}",
            rule_sem.lhs_undefined_terms
        );
    }

    // Recur: if any RHS term is undefined, add a child for each possible next
    // rule. Also track the height of the tree.
    let mut children: HashMap<BoundVar, Vec<RuleTree>> = HashMap::new();
    let mut max_height = 0;

    // TODO: need more complicated logic for multiple undefined terms
    assert!(
        rule_sem.rhs_undefined_terms.len() <= 1,
        "too many undefined terms: {}",
        rule_sem
            .rhs_undefined_terms
            .iter()
            .map(|t| t.name.clone())
            .collect::<Vec<String>>()
            .join(", ")
    );

    for t in rule_sem
        .rhs_undefined_terms
        .clone()
        .into_iter()
        .unique_by(|x| x.name.clone())
    {
        let next_rules = rules_with_lhs_root(&t.name, termenv, typeenv);

        // Since we are not at a leaf node (because there are undefined terms
        // on the RHS), we need next rules for any of the terms currently
        // undefined (which by definition lack annotations).
        assert!(
            !next_rules.is_empty(),
            "Missing annotation or next rules for unknown term  {:?}",
            t.name
        );
        let mut subtrees = vec![];
        for next_rule in rules_with_lhs_root(&t.name, termenv, typeenv) {
            let child =
                build_rule_tree_rec(ctx, &next_rule, termenv, typeenv, depth + 1, max_depth);
            if child.height > max_height {
                max_height = child.height;
            }
            subtrees.push(child);
        }
        children.insert(t.ret.clone(), subtrees);
    }

    RuleTree {
        value: rule_sem,
        children,
        height: max_height + 1,
    }
}

/// Enumerate all paths from root to leaves. Note: this is not optimized for
/// efficiency, values are cloned for each path.
fn enumerate_paths_to_leaves_rec(
    tree: &RuleTree,
    prior_term: Option<UndefinedTerm>,
) -> Vec<RulePath> {
    // Leaf base case
    if tree.children.is_empty() {
        assert!(
            tree.value.lhs_undefined_terms.len() <= 1,
            "Unexpected LHS undefined terms: {:?}",
            tree.value.lhs_undefined_terms
        );
        let undefined_term_pairs = match prior_term {
            Some(rhs_term) => vec![(rhs_term, tree.value.lhs_undefined_terms[0].clone())],
            None => vec![],
        };
        return vec![RulePath {
            rules: vec![tree.value.clone()],
            undefined_term_pairs,
        }];
    }
    let mut all_paths = vec![];
    // For now, assume there is at most one undefined term per RHS
    assert!(tree.children.len() <= 1);
    for (term, children) in &tree.children {
        for child in children {
            let rhs_undefined_term = tree
                .value
                .rhs_undefined_terms
                .iter()
                .find(|x| x.ret == *term);
            assert!(rhs_undefined_term.is_some());
            let paths = enumerate_paths_to_leaves_rec(child, rhs_undefined_term.cloned());
            for path in paths {
                let mut rules = path.rules.clone();
                rules.insert(0, tree.value.clone());
                let mut undefined_term_pairs = path.undefined_term_pairs.clone();
                match prior_term {
                    Some(ref rhs_term) => {
                        let lhs_undefined_term = child
                            .value
                            .lhs_undefined_terms
                            .iter()
                            .find(|x| x.ret == *term);
                        let new_pair = (rhs_term.clone(), lhs_undefined_term.unwrap().clone());
                        undefined_term_pairs.insert(0, new_pair);
                    }
                    None => (),
                };
                all_paths.push(RulePath {
                    rules,
                    undefined_term_pairs,
                })
            }
        }
    }
    all_paths
}

/// Enumerate all paths from root to leaves. Note: this is not optimized for
/// efficiency, values are cloned for each path.
pub fn enumerate_paths_to_leaves(tree: &RuleTree) -> Vec<RulePath> {
    let paths = enumerate_paths_to_leaves_rec(tree, None);
    for path in &paths {
        assert_eq!(path.rules.len(), path.undefined_term_pairs.len() + 1);
    }
    paths
}

pub fn build_rule_tree_from_root(
    rule: &Rule,
    termenv: &TermEnv,
    typeenv: &TypeEnv,
    annotationenv: &AnnotationEnv,
    typesols: &HashMap<RuleId, Solution>,
    width: usize,
) -> RuleTree {
    let mut ctx = Context::new(termenv, typeenv, annotationenv, typesols, width);
    build_rule_tree_rec(&mut ctx, rule, termenv, typeenv, 0, 20)
}

pub fn rules_with_lhs_root(name: &str, termenv: &TermEnv, typeenv: &TypeEnv) -> Vec<Rule> {
    termenv
        .rules
        .iter()
        .filter_map(|rule| {
            if pattern_term_name(rule.lhs.clone(), termenv, typeenv) == name {
                Some(rule.clone())
            } else {
                None
            }
        })
        .collect()
}

pub fn verify_rules_with_lhs_contains(
    root: &str,
    termenv: &TermEnv,
    typeenv: &TypeEnv,
    annotationenv: &AnnotationEnv,
    typesols: &HashMap<RuleId, Solution>,
) -> VerificationResult {
    let mut total_result = VerificationResult::Success;
    for width in all_query_widths() {
        let result = verify_rules_for_type_with_lhs_contains(
            root,
            termenv,
            typeenv,
            annotationenv,
            typesols,
            width,
        );
        if result != VerificationResult::Success {
            total_result = result;
        }
    }
    return total_result;
}

pub fn verify_rules_with_lhs_root(
    root: &str,
    termenv: &TermEnv,
    typeenv: &TypeEnv,
    annotationenv: &AnnotationEnv,
    typesols: &HashMap<RuleId, Solution>,
) -> VerificationResult {
    let mut total_result = VerificationResult::Success;
    for width in all_query_widths() {
        let result = verify_rules_for_type_with_lhs_root(
            root,
            termenv,
            typeenv,
            annotationenv,
            typesols,
            width,
        );
        if result != VerificationResult::Success {
            total_result = result;
        }
    }
    return total_result;
}

pub fn verify_rules_for_type_with_lhs_contains(
    name: &str,
    termenv: &TermEnv,
    typeenv: &TypeEnv,
    annotationenv: &AnnotationEnv,
    typesols: &HashMap<RuleId, Solution>,
    width: usize,
) -> VerificationResult {
    verify_rules_for_type_wih_rule_filter(
        termenv,
        typeenv,
        annotationenv,
        typesols,
        width,
        |rule, termenv, typeenv| pattern_contains_termname(&rule.lhs, name, termenv, typeenv),
    )
}

pub fn verify_rules_for_type_with_lhs_root(
    root: &str,
    termenv: &TermEnv,
    typeenv: &TypeEnv,
    annotationenv: &AnnotationEnv,
    typesols: &HashMap<RuleId, Solution>,
    width: usize,
) -> VerificationResult {
    verify_rules_for_type_wih_rule_filter(
        termenv,
        typeenv,
        annotationenv,
        typesols,
        width,
        |rule, termenv, typeenv| pattern_term_name(rule.lhs.clone(), termenv, typeenv) == root,
    )
}

pub fn verify_rules_for_type_wih_rule_filter(
    termenv: &TermEnv,
    typeenv: &TypeEnv,
    annotationenv: &AnnotationEnv,
    typesols: &HashMap<RuleId, Solution>,
    width: usize,
    filter: impl Fn(&Rule, &TermEnv, &TypeEnv) -> bool,
) -> VerificationResult {
    let mut rules_checked = 0;
    for rule in &termenv.rules {
        if !filter(&rule, termenv, typeenv) {
            println!("skipping rule that doesn't meet filter");
            continue;
        }
        let ctx = Context::new(termenv, typeenv, annotationenv, typesols, width);
        if ctx.typesols.get(&rule.id).is_none() {
            continue;
        }
        let sol = &ctx.typesols[&rule.id];
        let rule_sem = RuleSemantics {
            lhs: sol.lhs.clone(),
            rhs: sol.rhs.clone(),
            assumptions: sol.assumptions.clone(),
            quantified_vars: sol.quantified_vars.clone(),
            free_vars: sol.free_vars.clone(),
            tyctx: sol.tyctx.to_owned(),
            lhs_undefined_terms: vec![],
            rhs_undefined_terms: vec![],
        };
        let result = run_solver(rule_sem, width);
        rules_checked += 1;
        if result != VerificationResult::Success {
            return result;
        }
    }
    if rules_checked > 0 {
        VerificationResult::Success
    } else {
        panic!("No rules checked!")
    }
}

fn pattern_term_name(pattern: Pattern, termenv: &TermEnv, typeenv: &TypeEnv) -> String {
    match pattern {
        Pattern::Term(_, termid, _) => {
            let term = &termenv.terms[termid.index()];
            typeenv.syms[term.name.index()].clone()
        }
        _ => unreachable!("Must be term"),
    }
}
