//! Compilation process, from AST to Sema to Sequences of Insts.

use crate::error::Error;
use crate::{ast, codegen, sema};

pub fn compile(defs: &ast::Defs) -> Result<String, Vec<Error>> {
    let mut typeenv = sema::TypeEnv::from_ast(defs)?;
    let termenv = sema::TermEnv::from_ast(&mut typeenv, defs)?;
    let codegen = codegen::Codegen::compile(&typeenv, &termenv);
    Ok(codegen.generate_rust())
}
