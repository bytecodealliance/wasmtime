use std::collections::HashMap;

use anyhow::{Result, format_err};
use clap::Parser;
use cranelift_codegen_meta::{generate_isle, isle::get_isle_compilations};
use cranelift_isle_veri::debug::print_expansion;
use cranelift_isle_veri::expand::{Chaining, Expander};
use cranelift_isle_veri::program::Program;

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

    /// Term to expand.
    #[arg(long, required = true)]
    term_name: String,

    /// Whether to disable pruning of infeasible expansions.
    #[arg(long)]
    no_prune_infeasible: bool,

    /// Term names to chain.
    #[arg(long, value_name = "TERM_NAME")]
    chain: Vec<String>,

    /// Whether to enable maximal chaining.
    #[arg(long)]
    maximal_chaining: bool,

    /// Maximum rules: only chain terms with at most this many rules.
    #[arg(long, default_value = "0")]
    max_rules: usize,

    /// Terms to exclude from chaining.
    #[arg(long, value_name = "TERM_NAME")]
    exclude_chain: Vec<String>,
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

    // Configure chaining.
    let term_rule_sets: HashMap<_, _> = prog.build_trie()?.into_iter().collect();
    let mut chaining = Chaining::new(&prog, &term_rule_sets)?;
    chaining.chain_terms(&opts.chain)?;
    chaining.set_default(opts.maximal_chaining);
    chaining.set_max_rules(opts.max_rules);
    chaining.exclude_chain_terms(&opts.exclude_chain)?;

    // Build expansions.
    let mut expander = Expander::new(&prog, &term_rule_sets, chaining);
    expander.add_root_term_name(&opts.term_name)?;
    expander.set_prune_infeasible(!opts.no_prune_infeasible);
    expander.expand();

    // Report.
    let expansions = expander.expansions();
    println!("expansions = {}", expansions.len());
    for expansion in expansions {
        print_expansion(&prog, expansion);
    }

    Ok(())
}
