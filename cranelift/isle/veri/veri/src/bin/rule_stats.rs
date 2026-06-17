use anyhow::{Result, format_err};
use clap::Parser;
use cranelift_codegen_meta::{generate_isle, isle::get_isle_compilations};
use cranelift_isle::sema::{Pattern, Rule, RuleId, Term};
use cranelift_isle_veri::program::Program;
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
    let expand_internal_extractors = false;
    let prog = Program::from_files(&inputs, expand_internal_extractors)?;

    // Stats.
    let rules = prog.rules_by_term();
    let mut total_num_terms = 0;
    let mut total_num_rules = 0;
    let mut term_class_counts: HashMap<String, usize> = HashMap::new();
    for term in &prog.termenv.terms {
        let rule_ids = rules.get(&term.id).cloned().unwrap_or_default();
        let class = classify_term(&prog, term, &rule_ids);
        *term_class_counts.entry(class.clone()).or_default() += 1;

        total_num_terms += 1;
        total_num_rules += rule_ids.len();

        println!("{}\t{}\t{}", prog.term_name(term.id), class, rule_ids.len());
    }

    println!();
    println!("TOTAL: num_terms = {total_num_terms}",);
    println!("TOTAL: num_rules = {total_num_rules}");
    for (class, count) in term_class_counts {
        println!("TOTAL: class:{class} = {count}");
    }

    Ok(())
}

fn classify_term(prog: &Program, term: &Term, rule_ids: &[RuleId]) -> String {
    if term.is_enum_variant() {
        return "enum_variant".to_string();
    }

    if term.has_external_constructor() || term.has_external_extractor() {
        return "external".to_string();
    }

    if term.has_extractor() {
        return "extractor".to_string();
    }

    assert!(term.has_constructor());

    if rule_ids.len() == 1 && is_macro_rule(prog.rule(rule_ids[0])) {
        return "macro".to_string();
    }

    "constructor".to_string()
}

fn is_macro_rule(rule: &Rule) -> bool {
    if !rule.iflets.is_empty() {
        return false;
    }

    for arg in &rule.args {
        if !is_any_pattern(arg) {
            return false;
        }
    }

    true
}

fn is_any_pattern(pattern: &Pattern) -> bool {
    match pattern {
        Pattern::BindPattern(_, _, subpat) => is_any_pattern(subpat),
        Pattern::Wildcard(_) => true,
        _ => false,
    }
}
