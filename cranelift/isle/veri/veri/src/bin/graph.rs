use anyhow::{Result, format_err};
use clap::Parser;
use cranelift_codegen_meta::{generate_isle, isle::get_isle_compilations};
use cranelift_isle::trie_again::BindingId;
use cranelift_isle_veri::{
    debug::binding_string,
    expand::{Chaining, Expander, Expansion},
    program::Program,
};
use std::collections::HashMap;

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

    /// Filter to expansions involving this rule.
    #[arg(long, required = true)]
    rule: String,
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
    let _ = env_logger::try_init();
    let opts = Opts::parse();

    // Read ISLE inputs.
    let inputs = opts.isle_input_files()?;
    let root_term = if opts.name != "opt" {
        "lower"
    } else {
        "simplify"
    };
    let expand_internal_extractors = false;
    let prog = Program::from_files(&inputs, expand_internal_extractors)?;
    let term_rule_sets: HashMap<_, _> = prog.build_trie()?.into_iter().collect();

    // Lookup target rule.
    let rule = prog.get_rule_by_identifier(&opts.rule).ok_or(format_err!(
        "unknown rule: {rule_name}",
        rule_name = opts.rule
    ))?;

    // Generate expansions.
    // TODO(mbm): don't hardcode the expansion configuration
    let chaining = Chaining::new(&prog, &term_rule_sets)?;
    let mut expander = Expander::new(&prog, &term_rule_sets, chaining);
    expander.add_root_term_name(root_term)?;
    expander.set_prune_infeasible(true);
    expander.expand();

    // Process expansions.
    for expansion in expander.expansions() {
        if !expansion.rules.contains(&rule.id) {
            continue;
        }
        expansion_graph(expansion, &prog);
    }

    Ok(())
}

fn expansion_graph(expansion: &Expansion, prog: &Program) {
    // Header.
    println!("graph {{");
    println!("\tnode [shape=box, fontname=monospace];");

    // Binding nodes.
    let lookup_binding =
        |binding_id: BindingId| expansion.bindings[binding_id.index()].clone().unwrap();
    for (i, binding) in expansion.bindings.iter().enumerate() {
        if let Some(binding) = binding {
            println!(
                "\tb{i} [label=\"{i}: {}\"];",
                binding_string(binding, expansion.term, prog, lookup_binding)
            );
        }
    }

    // Edges.
    for (i, binding) in expansion.bindings.iter().enumerate() {
        if let Some(binding) = binding {
            for source in binding.sources() {
                println!("\tb{i} -- b{j};", j = source.index());
            }
        }
    }

    println!("}}");
}
