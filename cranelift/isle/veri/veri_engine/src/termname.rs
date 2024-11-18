use cranelift_isle as isle;
use isle::sema::{Pattern, TermEnv, TypeEnv};

/// Check whether the pattern (the LHS term) contains a given term name,
/// including in any subterms.
pub fn pattern_contains_termname(
    pat: &Pattern,
    name: &str,
    termenv: &TermEnv,
    typeenv: &TypeEnv,
) -> bool {
    match pat {
        Pattern::BindPattern(..)
        | Pattern::Var(..)
        | Pattern::ConstInt(..)
        | Pattern::ConstBool(..)
        | Pattern::ConstPrim(..)
        | Pattern::Wildcard(..) => false,
        Pattern::Term(_, termid, arg_patterns) => {
            let term = &termenv.terms[termid.index()];
            let term_name = &typeenv.syms[term.name.index()];
            (term_name == name)
                || arg_patterns
                    .iter()
                    .any(|p| pattern_contains_termname(p, name, termenv, typeenv))
        }
        Pattern::And(_, children) => children
            .iter()
            .any(|p| pattern_contains_termname(p, name, termenv, typeenv)),
    }
}
