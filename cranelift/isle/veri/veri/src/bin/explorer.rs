use anyhow::{Result, format_err};
use clap::Parser;
use cranelift_codegen_meta::{generate_isle, isle::get_isle_compilations};
use cranelift_isle_veri::{
    expand::{Chaining, Expander},
    explorer::ExplorerWriter,
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

    /// Output directory for explorer files.
    #[arg(long, required = true)]
    output_dir: std::path::PathBuf,

    /// Whether to enable graph generation.
    #[arg(long, env = "ISLE_EXPLORER_GRAPHS")]
    graphs: bool,
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

    // Generate expansions.
    // TODO(mbm): don't hardcode the expansion configuration
    let chaining = Chaining::new(&prog, &term_rule_sets)?;
    let mut expander = Expander::new(&prog, &term_rule_sets, chaining);
    expander.add_root_term_name(root_term)?;
    expander.set_prune_infeasible(true);
    expander.expand();

    // Generate explorer.
    let mut explorer_writer = ExplorerWriter::new(
        opts.output_dir,
        &prog,
        expander.chaining(),
        expander.expansions(),
    );
    if opts.graphs {
        explorer_writer.enable_graphs();
    }
    explorer_writer.write()?;

    Ok(())
}
