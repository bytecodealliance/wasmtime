use std::collections::HashMap;

use anyhow::{Result, format_err};
use clap::Parser;
use cranelift_codegen_meta::{generate_isle, isle::get_isle_compilations};
use cranelift_isle_veri::{program::Program, reachability::Reachability};

#[derive(Parser)]
struct Opts {
    /// Name of the ISLE compilation.
    #[arg(long, required = true)]
    name: String,

    /// Path to codegen crate directory.
    #[arg(long, required = true)]
    codegen_crate_dir: std::path::PathBuf,

    /// Working directory.
    #[arg(long, required = true)]
    work_dir: std::path::PathBuf,

    /// Whether to disable expansion of internal extractors.
    #[arg(long)]
    no_expand_internal_extractors: bool,
}

impl Opts {
    fn isle_input_files(&self) -> Result<Vec<std::path::PathBuf>> {
        // Generate ISLE files.
        let gen_dir = &self.work_dir;
        generate_isle(gen_dir)?;

        // Lookup ISLE compilations.
        let compilations = get_isle_compilations(&self.codegen_crate_dir, gen_dir);

        // Return inputs from the matching compilation, if any.
        Ok(compilations
            .lookup(&self.name)
            .ok_or(format_err!("unknown ISLE compilation: {}", self.name))?
            .paths()?)
    }
}

fn main() -> Result<()> {
    let opts = Opts::parse();

    // Read ISLE inputs.
    let inputs = opts.isle_input_files()?;
    let prog = Program::from_files(&inputs, !opts.no_expand_internal_extractors)?;

    // Derive rule sets.
    let term_rule_sets: HashMap<_, _> = prog.build_trie()?.into_iter().collect();
    println!("#term_rule_sets = {}", term_rule_sets.len());

    // Construct reachability.
    let reach = Reachability::build(&term_rule_sets);

    for (term_id, rule_set) in &term_rule_sets {
        let cyclic = reach.is_cyclic(*term_id);
        let reachable = reach.reachable(*term_id);

        println!("term = {}", prog.term_name(*term_id));
        println!("\tcyclic = {cyclic}");
        println!("\t#rules = {}", rule_set.rules.len());
        println!("\t#reachable = {}", reachable.len());
        for reach_term_id in reachable {
            println!("\treachable = {}", prog.term_name(*reach_term_id));
        }
    }

    Ok(())
}
