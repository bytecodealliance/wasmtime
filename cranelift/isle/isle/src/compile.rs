//! Compilation process, from AST to Sema to Sequences of Insts.

use std::path::Path;

use crate::error::Errors;
use crate::{ast, codegen, sema, trie};

/// Compile the given AST definitions into Rust source code.
pub fn compile(defs: &ast::Defs, options: &codegen::CodegenOptions) -> Result<String, Errors> {
    let mut typeenv = sema::TypeEnv::from_ast(defs)?;
    let termenv = sema::TermEnv::from_ast(
        &mut typeenv,
        defs,
        /* expand_internal_extractors */ true,
    )?;
    crate::overlap::check(&mut typeenv, &termenv)?;
    let tries = trie::build_tries(&termenv);
    Ok(codegen::codegen(&typeenv, &termenv, &tries, options))
}

/// Compile the given files into Rust source code.
pub fn from_files<P: AsRef<Path>>(
    inputs: impl IntoIterator<Item = P>,
    options: &codegen::CodegenOptions,
) -> Result<String, Errors> {
    let lexer = crate::lexer::Lexer::from_files(inputs)?;
    let defs = crate::parser::parse(lexer)?;
    compile(&defs, options)
}

/// Construct the ISLE type and term environments for further analysis
/// (i.e., verification), without going all the way through codegen.
pub fn create_envs(defs: &ast::Defs) -> Result<(sema::TypeEnv, sema::TermEnv), Errors> {
    let mut typeenv = sema::TypeEnv::from_ast(defs)?;
    // We want to allow annotations on terms with internal extractors,
    // so we avoid expanding them within the sema rules.
    let termenv = sema::TermEnv::from_ast(
        &mut typeenv,
        defs,
        /* expand_internal_extractors */ false
    )?;
    Ok((typeenv, termenv))
}
