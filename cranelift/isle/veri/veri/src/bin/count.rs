use std::collections::{BTreeSet, HashMap, HashSet};

use anyhow::{Result, format_err};
use clap::Parser;
use cranelift_codegen_meta::{generate_isle, isle::get_isle_compilations};
use cranelift_isle::{
    sema::TermId,
    trie_again::{BindingId, Rule, RuleSet},
};
use cranelift_isle_veri::{
    program::Program,
    reachability::{self, Reachability},
};

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

    /// Term to count.
    #[arg(long, required = true)]
    term_name: String,

    /// Maximum rules: only expand terms with at most this many rules.
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

    // Derive rule sets.
    let term_rule_sets: HashMap<_, _> = prog.build_trie()?.into_iter().collect();

    // Lookup term to count.
    let root_term_id = prog
        .get_term_by_name(opts.term_name.as_str())
        .ok_or(format_err!("unknown term {}", opts.term_name))?;
    println!("term = {}", opts.term_name);
    println!("id = {}", root_term_id.index());

    // Count expansions.
    let mut expansion_counter = ExpansionCounter::new(&prog, &term_rule_sets);
    expansion_counter.enable_expansion(root_term_id);
    if opts.max_rules > 0 {
        expansion_counter.set_max_rules(opts.max_rules);
    }
    for exclude_term_name in &opts.exclude_chain {
        let exclude_term_id = prog
            .get_term_by_name(exclude_term_name)
            .ok_or(format_err!("unknown term {exclude_term_name}"))?;
        expansion_counter.disable_expansion(exclude_term_id);
    }

    let n = expansion_counter.term(root_term_id, "");
    println!("expansions = {n}");

    Ok(())
}

struct ExpansionCounter<'a> {
    prog: &'a Program,
    term_rule_sets: &'a HashMap<TermId, RuleSet>,
    reach: Reachability,

    enable_expansion: HashSet<TermId>,
    disable_expansion: HashSet<TermId>,
    max_rules: usize,
}

impl<'a> ExpansionCounter<'a> {
    fn new(prog: &'a Program, term_rule_sets: &'a HashMap<TermId, RuleSet>) -> Self {
        Self {
            prog,
            term_rule_sets,
            reach: Reachability::build(term_rule_sets),

            enable_expansion: HashSet::new(),
            disable_expansion: HashSet::new(),
            max_rules: usize::MAX,
        }
    }

    fn term(&mut self, term_id: TermId, indent: &str) -> usize {
        println!(
            "{indent}> {term_name}",
            term_name = self.prog.term_name(term_id)
        );

        let n = if !self.may_expand(term_id) {
            1
        } else {
            let rule_set = &self.term_rule_sets[&term_id];
            self.rule_set(rule_set, indent)
        };

        if n > 1 {
            println!(
                "{indent}< {term_name} = {n}",
                term_name = self.prog.term_name(term_id)
            );
        }

        n
    }

    fn enable_expansion(&mut self, term_id: TermId) {
        self.enable_expansion.insert(term_id);
    }

    fn disable_expansion(&mut self, term_id: TermId) {
        self.disable_expansion.insert(term_id);
    }

    fn set_max_rules(&mut self, max_rules: usize) {
        self.max_rules = max_rules;
    }

    fn may_expand(&mut self, term_id: TermId) -> bool {
        if !self.term_rule_sets.contains_key(&term_id) {
            return false;
        }

        if self.reach.is_cyclic(term_id) {
            return false;
        }

        if self.enable_expansion.contains(&term_id) {
            return true;
        }

        if self.disable_expansion.contains(&term_id) {
            return false;
        }

        let rule_set = &self.term_rule_sets[&term_id];
        if rule_set.rules.len() > self.max_rules {
            return false;
        }

        true
    }

    fn rule_set(&mut self, rule_set: &RuleSet, indent: &str) -> usize {
        let mut n = 0;
        for rule in &rule_set.rules {
            let r = self.rule(rule_set, rule, indent);
            n += r;
            println!(
                "{indent}n={n} r={r} rule={}",
                rule.pos.pretty_print_line(&self.prog.files)
            );
        }
        n
    }

    fn rule(&mut self, rule_set: &RuleSet, rule: &Rule, indent: &str) -> usize {
        let binding_ids = rule_bindings(rule_set, rule);
        let mut n = 1;
        for binding_id in binding_ids {
            let binding = &rule_set.bindings[binding_id.index()];
            if let Some(term_id) = reachability::binding_used_term(binding) {
                n *= self.term(term_id, &format!("{indent}.\t"));
            }
        }
        n
    }
}

fn rule_bindings(rule_set: &RuleSet, rule: &Rule) -> BTreeSet<BindingId> {
    // TODO(mbm): duplicates logic in expand::Application

    // Initialize stack of bindings used directly by the rule.
    let mut stack = Vec::new();

    // Result binding.
    stack.push(rule.result);

    // Constraints and equality.
    for i in 0..rule_set.bindings.len() {
        let binding_id = i.try_into().unwrap();

        if rule.get_constraint(binding_id).is_some() {
            stack.push(binding_id);
        }

        if let Some(equal_binding_id) = rule.equals.find(binding_id) {
            stack.push(equal_binding_id);
        }
    }

    // TODO(mbm): iterators, prio?

    // Impure.
    stack.extend(&rule.impure);

    // Collect dependencies.
    let mut binding_ids = BTreeSet::new();
    while let Some(binding_id) = stack.pop() {
        if binding_ids.contains(&binding_id) {
            continue;
        }
        binding_ids.insert(binding_id);

        let binding = &rule_set.bindings[binding_id.index()];
        stack.extend(binding.sources());
    }

    binding_ids
}
