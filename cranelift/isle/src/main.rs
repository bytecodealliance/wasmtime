#![allow(dead_code)]

use std::io::stdin;
use std::io::Read;

mod ast;
mod codegen;
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
    let automata = compile::compile(&defs)?;
    println!("automata: {:?}", automata);
    Ok(())
}
