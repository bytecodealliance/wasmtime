//! Compilation process, from AST to Sema to Sequences of Insts.

use crate::error::Result;
use crate::{ast, codegen, sema, trie};

/// Compile the given AST definitions into Rust source code.
pub fn compile(defs: &ast::Defs) -> Result<String> {
    let mut typeenv = sema::TypeEnv::from_ast(defs)?;
    let termenv = sema::TermEnv::from_ast(&mut typeenv, defs)?;
    let tries = trie::build_tries(&typeenv, &termenv);
    Ok(codegen::codegen(&typeenv, &termenv, &tries))
}
