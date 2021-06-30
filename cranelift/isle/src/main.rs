#![allow(dead_code)]

use std::io::stdin;
use std::io::Read;

mod ast;
mod compile;
mod error;
mod ir;
mod lexer;
mod parser;
mod sema;

fn main() -> Result<(), error::Error> {
    let _ = env_logger::try_init();
    let mut input = String::new();
    stdin().read_to_string(&mut input)?;
    let mut parser = parser::Parser::new("<stdin>", &input[..]);
    let defs = parser.parse_defs()?;
    let mut compiler = compile::Compiler::new(&defs)?;
    compiler.build_sequences()?;
    compiler.collect_tree_summaries()?;

    for seq in compiler.to_sequences() {
        println!("---\nsequence\n---\n{:?}\n", seq);
    }
    Ok(())
}
