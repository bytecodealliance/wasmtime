//! Compilation process, from AST to Sema to Sequences of Insts.

use crate::error::Result;
use crate::sema::{TermEnv, TypeEnv};
use crate::{ast, codegen, sema, trie};

/// Compile the given AST definitions into Rust source code.
pub fn compile(defs: &ast::Defs) -> Result<String> {
    let mut typeenv = sema::TypeEnv::from_ast(defs)?;
    let termenv = sema::TermEnv::from_ast(
        &mut typeenv,
        defs,
        /* expand_internal_extractors */ true,
    )?;
    let tries = trie::build_tries(&typeenv, &termenv);
    Ok(codegen::codegen(&typeenv, &termenv, &tries))
}

/// Construct the ISLE type and term environments for further analysis
/// (i.e., verification), without going all the way through codegen.
pub fn create_envs(defs: &ast::Defs) -> Result<(TypeEnv, TermEnv)> {
    let mut typeenv = sema::TypeEnv::from_ast(defs)?;
    // We want to allow annotations on terms with internal extractors,
    // so we avoid expanding them within the sema rules.
    let termenv = sema::TermEnv::from_ast(
        &mut typeenv,
        defs,
        /* expand_internal_extractors */ false,
    )?;
    Ok((typeenv, termenv))
}
