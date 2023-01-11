use cranelift_isle as isle;
use isle::compile::create_envs;
use isle::sema::{TermEnv, TypeEnv};
use std::path::PathBuf;

pub mod interp;
pub mod rule_tree;
pub mod solver;
pub mod termname;
pub mod type_check;
pub mod type_inference;

pub const REG_WIDTH: usize = 64;

/// Given a file, lexes and parses the file to an ISLE term and type environment tuple
pub fn isle_files_to_terms(files: &Vec<PathBuf>) -> (TypeEnv, TermEnv) {
    let lexer = isle::lexer::Lexer::from_files(files).unwrap();
    parse_isle_to_terms(lexer)
}

/// Produces the two ISLE-defined structs with type and term environments
pub fn parse_isle_to_terms(lexer: isle::lexer::Lexer) -> (TypeEnv, TermEnv) {
    // Parses to an AST, as a list of definitions
    let defs = isle::parser::parse(lexer).expect("should parse");

    // Produces environments including terms, rules, and maps from symbols and
    // names to types
    create_envs(&defs).unwrap()
}
