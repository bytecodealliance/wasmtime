//! Type checking of term specifications in isolation, to help identify
//! vacuously false type instantiations.
//!
//! Each term is checked on its own, against nothing but its own declared
//! types. The check needs no solver, so it is cheap enough to run as part of
//! every verification run, and to run as a test.

use std::collections::{BTreeMap, BTreeSet, HashSet};

use cranelift_isle::sema::TermId;

use crate::{
    program::Program,
    spec::Signature,
    type_inference::{self, Choice, type_constraint_system},
    veri::{Conditions, TermKind},
};

/// What went wrong when checking one term.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FindingKind {
    Conflict,
    Build,
}

/// A term whose spec does not type check on its own.
#[derive(Debug, Clone)]
pub struct Finding {
    pub kind: FindingKind,
    pub term: String,
    pub signature: Option<String>,
    pub message: String,
}

impl Finding {
    pub fn line(&self) -> String {
        match &self.signature {
            Some(signature) => format!(
                "{term}\t{signature}\t{message}",
                term = self.term,
                message = self.message
            ),
            None => format!(
                "{term}\t{message}",
                term = self.term,
                message = self.message
            ),
        }
    }
}

/// Type check each of `terms` against its own declared types.
///
/// Terms with no spec are skipped, so the caller may pass the whole set of
/// terms a run reaches without filtering it first.
///
/// A term with an `(instantiate ...)` set gets the stronger check: those
/// signatures are concrete, and each becomes one branch of the constraint
/// system, so every one of them must be solvable. Without one, the argument
/// types come from models alone and are often polymorphic, leaving inference
/// underconstrained -- which is not an error, but does mean only outright
/// conflicts are detectable.
pub fn check(prog: &Program, terms: &BTreeSet<TermId>) -> Vec<Finding> {
    let mut findings = Vec::new();
    for &term in terms {
        if !prog.specenv.has_spec(term) {
            continue;
        }
        findings.extend(check_term(prog, term));
    }
    findings
}

fn check_term(prog: &Program, term: TermId) -> Vec<Finding> {
    let term_name = prog.term_name(term).to_string();

    // A term declared with both a constructor and an extractor is used in both
    // directions, and its spec need only make sense in one of them: a `match`
    // clause over `result`, for instance, is well formed for an extractor,
    // where the result is an input, and not for a constructor. Report findings
    // only when no direction checks out, and then the first direction's, since
    // there is no basis for preferring one set of diagnostics over the other.
    let term_data = prog.term(term);
    let mut kinds = Vec::new();
    if term_data.has_constructor() {
        kinds.push(TermKind::Constructor);
    }
    if term_data.has_extractor() {
        kinds.push(TermKind::Extractor);
    }

    let mut first: Option<Vec<Finding>> = None;
    for kind in kinds {
        let findings = check_term_kind(prog, term, kind, &term_name);
        if findings.is_empty() {
            return Vec::new();
        }
        first.get_or_insert(findings);
    }
    first.unwrap_or_default()
}

fn check_term_kind(prog: &Program, term: TermId, kind: TermKind, term_name: &str) -> Vec<Finding> {
    let term_name = term_name.to_string();
    let conditions = match Conditions::from_term(term, kind, prog) {
        Ok(conditions) => conditions,
        Err(err) => {
            return vec![Finding {
                kind: FindingKind::Build,
                term: term_name,
                signature: None,
                message: format!("{err:#}"),
            }];
        }
    };

    // The constraint system branches over the term's declared instantiations,
    // so a solution normally carries the signature it belongs to and failures
    // can be attributed to individual signatures. A conflict found before any
    // branching carries no signature: it comes from the constraints every
    // candidate shares, so no choice of signature can rescue it.
    let system = type_constraint_system(&conditions);
    let solutions = type_inference::Solver::new().solve(&system);

    let mut any_solved = false;
    let mut solved: HashSet<String> = HashSet::new();
    let mut failures: BTreeMap<String, String> = BTreeMap::new();
    let mut common_failure: Option<String> = None;
    for solution in &solutions {
        let signature = solution
            .choices
            .iter()
            .find_map(|choice| match choice {
                Choice::TermInstantiation(choice_term, sig) if *choice_term == term => Some(sig),
                _ => None,
            })
            .map(signature_string);

        match &solution.status {
            // Underconstrained is not a failure here.
            type_inference::Status::Solved | type_inference::Status::Underconstrained => {
                any_solved = true;
                if let Some(signature) = signature {
                    solved.insert(signature);
                }
            }
            type_inference::Status::Inapplicable(conflict)
            | type_inference::Status::TypeError(conflict) => {
                let diagnostic = conflict.diagnostic(&conditions, &prog.files);
                match signature {
                    Some(signature) => {
                        failures.entry(signature).or_insert(diagnostic);
                    }
                    None => {
                        common_failure.get_or_insert(diagnostic);
                    }
                }
            }
        }
    }

    if !any_solved && let Some(message) = common_failure {
        return vec![Finding {
            kind: FindingKind::Conflict,
            term: term_name,
            signature: None,
            message,
        }];
    }

    // A signature that appears in a failing branch may also appear in a solved
    // one, since other calls in the same conditions fork independently. Only
    // report the ones with no solution at all.
    failures
        .into_iter()
        .filter(|(signature, _)| !solved.contains(signature))
        .map(|(signature, message)| Finding {
            kind: FindingKind::Conflict,
            term: term_name.clone(),
            signature: Some(signature),
            message,
        })
        .collect()
}

fn signature_string(signature: &Signature) -> String {
    format!(
        "(args {args}) (ret {ret})",
        args = signature
            .args
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(" "),
        ret = signature.ret,
    )
}
