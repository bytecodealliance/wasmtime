use std::{
    collections::{BTreeSet, HashMap},
    fs::File,
    io::Write,
    path::{Path, PathBuf},
    str::FromStr,
    sync::Mutex,
    time::{self, Duration},
};

use anyhow::{Context as _, Error, Result, bail, format_err};
use cranelift_isle::{
    sema::{Term, TermId},
    trie_again::RuleSet,
};
use rayon::prelude::*;
use serde::Serialize;

use crate::{
    BUILD_PROFILE, GIT_VERSION,
    debug::{print_expansion, write_expansion},
    expand::{Chaining, Expander, Expansion},
    program::Program,
    solver::{Applicability, Dialect, Solver, Verification},
    type_inference::{self, Assignment, Choice, type_constraint_system},
    veri::Conditions,
};

const LOG_DIR: &str = ".veriisle";

#[derive(Debug, Clone, Copy)]
pub enum SolverBackend {
    Z3,
    CVC5,
}

impl SolverBackend {
    fn prog(&self) -> &str {
        match self {
            SolverBackend::Z3 => "z3",
            SolverBackend::CVC5 => "cvc5",
        }
    }

    fn all() -> Vec<Self> {
        vec![SolverBackend::Z3, SolverBackend::CVC5]
    }

    fn dialect(&self) -> Dialect {
        match self {
            SolverBackend::Z3 => Dialect::Z3,
            SolverBackend::CVC5 => Dialect::SMTLIB2,
        }
    }

    fn args(&self, timeout: Duration) -> Vec<String> {
        match self {
            SolverBackend::Z3 => vec![
                "-smt2".to_string(),
                "-in".to_string(),
                format!("-t:{}", timeout.as_millis()),
            ],
            SolverBackend::CVC5 => vec![
                "--incremental".to_string(),
                "--print-success".to_string(),
                format!("--tlimit-per={ms}", ms = timeout.as_millis()),
                "-".to_string(),
            ],
        }
    }
}

impl std::fmt::Display for SolverBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.prog())
    }
}

impl FromStr for SolverBackend {
    type Err = Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Ok(match s {
            "z3" => SolverBackend::Z3,
            "cvc5" => SolverBackend::CVC5,
            _ => bail!("unknown solver backend"),
        })
    }
}

#[derive(Debug, Clone)]
pub enum ExpansionPredicate {
    FirstRuleNamed,
    Specified,
    Tagged(String),
    Root(String),
    ContainsRule(String),
    Not(Box<ExpansionPredicate>),
    And(Box<ExpansionPredicate>, Box<ExpansionPredicate>),
}

impl FromStr for ExpansionPredicate {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        Ok(if let Some((p, q)) = s.split_once(',') {
            ExpansionPredicate::And(Box::new(p.parse()?), Box::new(q.parse()?))
        } else if let Some(p) = s.strip_prefix("not:") {
            ExpansionPredicate::Not(Box::new(p.parse()?))
        } else if s == "first-rule-named" {
            ExpansionPredicate::FirstRuleNamed
        } else if s == "specified" {
            ExpansionPredicate::Specified
        } else if let Some(tag) = s.strip_prefix("tag:") {
            ExpansionPredicate::Tagged(tag.to_string())
        } else if let Some(term) = s.strip_prefix("root:") {
            ExpansionPredicate::Root(term.to_string())
        } else if let Some(rule) = s.strip_prefix("rule:") {
            ExpansionPredicate::ContainsRule(rule.to_string())
        } else {
            bail!("invalid expansion predicate")
        })
    }
}

impl std::fmt::Display for ExpansionPredicate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExpansionPredicate::FirstRuleNamed => write!(f, "first-rule-named"),
            ExpansionPredicate::Specified => write!(f, "specified"),
            ExpansionPredicate::Tagged(tag) => write!(f, "tag:{tag}"),
            ExpansionPredicate::Root(term) => write!(f, "root:{term}"),
            ExpansionPredicate::ContainsRule(rule) => write!(f, "rule:{rule}"),
            ExpansionPredicate::Not(p) => write!(f, "not:{p}"),
            ExpansionPredicate::And(p, q) => write!(f, "{p},{q}"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Filter {
    include: bool,
    predicate: ExpansionPredicate,
}

impl Filter {
    fn new(include: bool, predicate: ExpansionPredicate) -> Self {
        Self { include, predicate }
    }

    fn include(predicate: ExpansionPredicate) -> Self {
        Self::new(true, predicate)
    }

    fn exclude(predicate: ExpansionPredicate) -> Self {
        Self::new(false, predicate)
    }
}

impl FromStr for Filter {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        let (include, p) = if let Some(p) = s.strip_prefix("include:") {
            (true, p)
        } else if let Some(p) = s.strip_prefix("exclude:") {
            (false, p)
        } else {
            (true, s)
        };
        Ok(Filter::new(include, p.parse()?))
    }
}

impl std::fmt::Display for Filter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let include = if self.include { "include" } else { "exclude" };
        write!(
            f,
            "{include}:{predicate}",
            include = include,
            predicate = self.predicate
        )
    }
}

#[derive(Debug, Clone)]
pub struct SolverRule {
    predicate: ExpansionPredicate,
    solver_backend: SolverBackend,
}

impl SolverRule {
    /// Build a rule that selects the solver backend for expansions with an
    /// explicit `solver_<name>` tag.
    fn solver_tag(solver_backend: SolverBackend) -> Self {
        let tag = format!("solver_{}", solver_backend);
        Self {
            predicate: ExpansionPredicate::Tagged(tag),
            solver_backend,
        }
    }

    /// Build rules for explicit selection of all solver backends.
    fn solver_tag_rules() -> Vec<Self> {
        SolverBackend::all()
            .iter()
            .map(|backend| Self::solver_tag(*backend))
            .collect()
    }
}

impl FromStr for SolverRule {
    type Err = Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        if let Some((backend, predicate)) = s.split_once('=') {
            Ok(Self {
                predicate: predicate.parse()?,
                solver_backend: backend.parse()?,
            })
        } else {
            bail!("invalid solver rule")
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    Inapplicable,
    Success,
    Unknown,
    Failure,
    ApplicabilityUnknown,
}

#[derive(Serialize)]
pub struct VerifyReport {
    pub verdict: Verdict,

    pub init_time: Duration,
    pub applicable_time: Duration,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verify_time: Option<Duration>,
}

#[derive(Serialize)]
pub struct TypeInstantationReport {
    pub choices: Vec<String>,
    pub verify: VerifyReport,
    pub duration: Duration,
}

#[derive(Serialize)]
pub struct ExpansionReport {
    pub id: usize,
    pub description: String,
    pub root: String,
    pub rules: Vec<String>,
    pub chained: Vec<String>,
    pub terms: Vec<String>,
    pub tags: Vec<String>,
    pub solver: String,
    /// Count of type instantiations that failed at type inference.
    pub failed_type_inference: usize,
    /// Solver reports from type instantiations.
    pub type_instantiations: Vec<TypeInstantationReport>,
    pub duration: Duration,
}

impl ExpansionReport {
    fn from_expansion(id: usize, expansion: &Expansion, prog: &Program) -> Result<Self> {
        // Description
        let description = expansion_description(expansion, prog)?;

        // Root term.
        let root = prog.term_name(expansion.term).to_string();

        // Tags
        let mut tags: Vec<_> = expansion.tags(prog).iter().cloned().collect();
        tags.sort();

        // Rules
        let mut rules = Vec::new();
        let mut chained = BTreeSet::new();
        for rule_id in &expansion.rules {
            let rule = prog.rule(*rule_id);
            rules.push(rule.identifier(&prog.tyenv, &prog.files));

            if rule.root_term != expansion.term {
                let root_term = prog.term_name(rule.root_term);
                if !chained.contains(&root_term) {
                    chained.insert(root_term);
                }
            }
        }

        // Terms
        let terms: BTreeSet<_> = expansion
            .terms(prog)
            .iter()
            .map(|term_id| prog.term_name(*term_id))
            .collect();

        Ok(Self {
            id,
            root,
            description,
            rules,
            chained: chained.iter().map(ToString::to_string).collect(),
            terms: terms.iter().map(ToString::to_string).collect(),
            tags,
            solver: Default::default(),
            failed_type_inference: 0,
            type_instantiations: Vec::new(),
            duration: Default::default(),
        })
    }
}

#[derive(Serialize)]
pub struct TermMetadata {
    pub name: String,
    pub class: String,
    pub has_spec: bool,
    pub tags: Vec<String>,
}

impl TermMetadata {
    fn from_term(term: &Term, prog: &Program) -> Self {
        let name = prog.term_name(term.id).to_string();
        let class = Self::classify_term(term);
        let has_spec = prog.specenv.has_spec(term.id);

        let tags_set = prog
            .specenv
            .term_tags
            .get(&term.id)
            .cloned()
            .unwrap_or_default();
        let mut tags: Vec<_> = tags_set.iter().cloned().collect();
        tags.sort();

        Self {
            name,
            class,
            has_spec,
            tags,
        }
    }

    fn from_prog(prog: &Program) -> Vec<Self> {
        let mut terms = Vec::new();
        for term in &prog.termenv.terms {
            terms.push(Self::from_term(term, prog));
        }
        terms
    }

    fn classify_term(term: &Term) -> String {
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

        "constructor".to_string()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureKind {
    Verification,
    ApplicabilityUnknown,
}

#[derive(Debug, Clone)]
pub struct FailureRecord {
    pub kind: FailureKind,
    pub expansion_id: usize,
    pub description: String,
    pub instantiation_index: usize,
    pub failure_path: PathBuf,
}

#[derive(Serialize)]
pub struct Report {
    build_profile: String,
    git_version: String,
    args: Vec<String>,
    filters: Vec<String>,
    default_solver: String,
    timeout: Duration,
    duration: Duration,
    num_threads: usize,
    terms: Vec<TermMetadata>,
    expansions: Vec<ExpansionReport>,
}

/// Runner orchestrates execution of the verification process over a set of
/// expansions.
pub struct Runner {
    prog: Program,
    term_rule_sets: HashMap<TermId, RuleSet>,

    /// Optional single root term to scope expansion to. If `None`, expansion is
    /// seeded from every term that has rules (all paths from all roots).
    root_term: Option<String>,
    filters: Vec<Filter>,
    default_solver_backend: SolverBackend,
    solver_rules: Vec<SolverRule>,
    timeout: Duration,
    log_dir: PathBuf,
    skip_solver: bool,
    results_to_log_dir: bool,
    debug: bool,
}

impl Runner {
    pub fn from_files(inputs: &Vec<PathBuf>) -> Result<Self> {
        let expand_internal_extractors = false;
        let prog = Program::from_files(inputs, expand_internal_extractors)?;
        let term_rule_sets: HashMap<_, _> = prog.build_trie()?.into_iter().collect();
        Ok(Self {
            prog,
            term_rule_sets,
            root_term: None,
            filters: Vec::new(),
            default_solver_backend: SolverBackend::CVC5,
            solver_rules: Vec::new(),
            timeout: Duration::from_secs(5),
            log_dir: PathBuf::from(LOG_DIR),
            results_to_log_dir: false,
            skip_solver: false,
            debug: false,
        })
    }

    pub fn set_root_term(&mut self, term: &str) {
        self.root_term = Some(term.to_string());
    }

    pub fn filter(&mut self, filter: Filter) {
        self.filters.push(filter);
    }

    pub fn filters(&mut self, filters: &[Filter]) {
        self.filters.extend(filters.iter().cloned());
    }

    pub fn include_first_rule_named(&mut self) {
        self.filters
            .push(Filter::include(ExpansionPredicate::FirstRuleNamed));
    }

    pub fn skip_tag(&mut self, tag: &str) {
        self.filters
            .push(Filter::exclude(ExpansionPredicate::Tagged(tag.to_string())));
    }

    pub fn target_rule(&mut self, id: &str) -> Result<()> {
        self.filters
            .push(Filter::include(ExpansionPredicate::ContainsRule(
                id.to_string(),
            )));
        Ok(())
    }

    // Configure the default solver to use if no solver rules apply.
    pub fn set_default_solver_backend(&mut self, solver_backend: SolverBackend) {
        self.default_solver_backend = solver_backend;
    }

    // Use the given solver backend for expansions that satisfy the given
    // predicate.  If multiple rules match, the earlier one is used. If none
    // match, the default is used.
    pub fn add_solver_rule(&mut self, solver_rule: SolverRule) {
        self.solver_rules.push(solver_rule);
    }

    // Configure rules for explicit solver selection based on `solver_<name>` tags.
    pub fn add_solver_tag_rules(&mut self) {
        self.solver_rules.extend(SolverRule::solver_tag_rules());
    }

    pub fn set_timeout(&mut self, timeout: Duration) {
        self.timeout = timeout;
    }

    pub fn set_log_dir(&mut self, path: PathBuf) {
        self.log_dir = path;
    }

    pub fn set_results_to_log_dir(&mut self, enabled: bool) {
        self.results_to_log_dir = enabled;
    }

    pub fn skip_solver(&mut self, skip: bool) {
        self.skip_solver = skip;
    }

    pub fn debug(&mut self, debug: bool) {
        self.debug = debug;
    }

    pub fn run(&self) -> Result<()> {
        // Clean log directory.
        if self.log_dir.exists() {
            std::fs::remove_dir_all(&self.log_dir)?;
        }

        // Start timer.
        let num_threads = rayon::current_num_threads();
        let start = time::Instant::now();

        // Generate expansions.
        // TODO(mbm): don't hardcode the expansion configuration
        let chaining = Chaining::new(&self.prog, &self.term_rule_sets)?;
        chaining.validate()?;
        let mut expander = Expander::new(&self.prog, &self.term_rule_sets, chaining);
        match &self.root_term {
            // Scope expansion to a single explicitly configured root term.
            Some(root_term) => expander.add_root_term_name(root_term)?,
            // Default: seed an expansion at every term that has rules, a
            // constructor, and an explicit specification. Terms without a spec
            // are not verified standalone: they are assumed to be part of some
            // real chain and are reached only by being chained (inlined) into a
            // specified root's expansion. Sub-terms reachable from another root
            // are deduplicated by `add_root`.
            None => {
                for &term_id in self.term_rule_sets.keys() {
                    if self.prog.term(term_id).has_constructor()
                        && self.prog.specenv.has_spec(term_id)
                    {
                        expander.add_root(term_id);
                    }
                }
            }
        }
        expander.set_prune_infeasible(true);
        expander.expand();

        // Process expansions.
        let expansions = expander.expansions();
        log::info!("expansions: {n}", n = expansions.len());

        let failures: Mutex<Vec<FailureRecord>> = Mutex::new(Vec::new());

        let mut expansion_reports = expansions
            .par_iter()
            .enumerate()
            .map(|(i, expansion)| -> Result<Option<ExpansionReport>> {
                // Skip?
                if !self.should_verify(expansion)? {
                    return Ok(None);
                }

                // Verify
                let expansion_log_dir = self.log_dir.join("expansions").join(format!("{:05}", i));
                let report =
                    self.verify_expansion(expansion, i, expansion_log_dir.clone(), &failures)?;

                Ok(Some(report))
            })
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();

        // End timer.
        let duration = start.elapsed();

        // Report failures, partitioned by kind.
        let failures = failures.into_inner().unwrap();
        let (verification_failures, applicability_unknowns): (Vec<_>, Vec<_>) = failures
            .into_iter()
            .partition(|f| f.kind == FailureKind::Verification);

        let format_line = |failure: &FailureRecord| -> String {
            format!(
                "#{id}\t{description}\t(instantiation {inst})\t{path}",
                id = failure.expansion_id,
                description = failure.description,
                inst = failure.instantiation_index,
                path = failure.failure_path.display(),
            )
        };

        if !verification_failures.is_empty() {
            let mut summary = Self::open_log_file(self.log_dir.clone(), "failures.out").ok();
            eprintln!(
                "=== VERIFICATION FAILURES ({n}) ===",
                n = verification_failures.len()
            );
            for failure in &verification_failures {
                let line = format_line(failure);
                eprintln!("FAILURE {line}");
                if let Some(f) = summary.as_mut() {
                    let _ = writeln!(f, "{line}");
                }
            }
            log::warn!(
                "verification failures: {n}",
                n = verification_failures.len()
            );
        }

        if !applicability_unknowns.is_empty() {
            let mut summary =
                Self::open_log_file(self.log_dir.clone(), "applicability_unknowns.out").ok();
            eprintln!(
                "=== APPLICABILITY UNKNOWN ({n}) ===",
                n = applicability_unknowns.len()
            );
            for failure in &applicability_unknowns {
                let line = format_line(failure);
                eprintln!("APPLICABILITY UNKNOWN {line}");
                if let Some(f) = summary.as_mut() {
                    let _ = writeln!(f, "{line}");
                }
            }
            log::warn!(
                "applicability unknowns: {n}",
                n = applicability_unknowns.len()
            );
        }

        // Prepare report
        expansion_reports.sort_by(|a, b| a.id.cmp(&b.id));
        let terms = TermMetadata::from_prog(&self.prog);
        let report = Report {
            build_profile: BUILD_PROFILE.to_string(),
            git_version: GIT_VERSION.to_string(),
            args: std::env::args().collect(),
            filters: self.filters.iter().map(ToString::to_string).collect(),
            default_solver: self.default_solver_backend.prog().to_string(),
            timeout: self.timeout,
            num_threads,
            duration,
            terms,
            expansions: expansion_reports,
        };

        // Write
        let output = Self::open_log_file(self.log_dir.clone(), "report.json")?;
        serde_json::to_writer_pretty(output, &report)?;

        Ok(())
    }

    fn should_verify(&self, expansion: &Expansion) -> Result<bool> {
        let mut verdict = None;
        for filter in self.filters.iter() {
            verdict = self.eval_filter(filter, expansion)?.or(verdict);
        }
        // Default to including an expansion unless an `exclude` filter matches
        // it. Because the last matching filter wins, an `include` filter can
        // still carve an exception back out of a broader `exclude`.
        Ok(verdict.unwrap_or(true))
    }

    fn eval_filter(&self, filter: &Filter, expansion: &Expansion) -> Result<Option<bool>> {
        Ok(if self.eval_predicate(&filter.predicate, expansion)? {
            Some(filter.include)
        } else {
            None
        })
    }

    fn eval_predicate(
        &self,
        predicate: &ExpansionPredicate,
        expansion: &Expansion,
    ) -> Result<bool> {
        Ok(match predicate {
            ExpansionPredicate::FirstRuleNamed => {
                let rule_id = expansion
                    .rules
                    .first()
                    .ok_or(format_err!("expansion should have at least one rule"))?;
                let rule = self.prog.rule(*rule_id);
                rule.name.is_some()
            }
            ExpansionPredicate::Specified => expansion
                .terms(&self.prog)
                .iter()
                .all(|term_id| self.prog.specenv.has_spec(*term_id)),
            ExpansionPredicate::Tagged(tag) => {
                let tags = expansion.tags(&self.prog);
                tags.contains(tag)
            }
            ExpansionPredicate::Root(term) => self.prog.term_name(expansion.term) == term,
            ExpansionPredicate::ContainsRule(identifier) => {
                let rule = self
                    .prog
                    .get_rule_by_identifier(&identifier)
                    .ok_or(format_err!("unknown rule '{identifier}'"))?;
                expansion.rules.contains(&rule.id)
            }
            ExpansionPredicate::Not(p) => !self.eval_predicate(p, expansion)?,
            ExpansionPredicate::And(p, q) => {
                self.eval_predicate(p, expansion)? && self.eval_predicate(q, expansion)?
            }
        })
    }

    fn verify_expansion(
        &self,
        expansion: &Expansion,
        id: usize,
        log_dir: std::path::PathBuf,
        failures: &Mutex<Vec<FailureRecord>>,
    ) -> Result<ExpansionReport> {
        let description = expansion_description(expansion, &self.prog)?;
        let start = time::Instant::now();

        // Results output.
        let mut output: Box<dyn Write> = if self.results_to_log_dir {
            log::info!("#{id}\t{description}");
            Box::new(Self::open_log_file(log_dir.clone(), "results.out")?)
        } else {
            Box::new(std::io::stdout())
        };

        writeln!(output, "#{id}\t{description}")?;
        if self.debug {
            print_expansion(&self.prog, expansion);
        }

        // Verification conditions.
        let conditions = Conditions::from_expansion(expansion, &self.prog)?;
        if self.debug {
            conditions.pretty_print(&self.prog);
        }

        // Type constraints.
        let system = type_constraint_system(&conditions);
        if self.debug {
            system.pretty_print();
        }

        // Infer types.
        let type_solver = type_inference::Solver::new();
        let solutions = type_solver.solve(&system);

        // Initialize report.
        let mut report = ExpansionReport::from_expansion(id, expansion, &self.prog)?;

        // Select solver.
        let solver_backend = self.select_solver_backend(expansion)?;
        report.solver = solver_backend.to_string();

        for (i, solution) in solutions.iter().enumerate() {
            let start_solution = time::Instant::now();

            // Show type assignment.
            let mut choices = Vec::new();
            for choice in &solution.choices {
                let choice = match choice {
                    Choice::TermInstantiation(term_id, sig) => {
                        format!("{term}{sig}", term = self.prog.term_name(*term_id))
                    }
                };
                writeln!(output, "\t{choice}")?;
                choices.push(choice);
            }
            writeln!(output, "\t\ttype solution status = {}", solution.status)?;
            if self.debug {
                println!("type assignment:");
                solution.assignment.pretty_print(&conditions);
            }

            match &solution.status {
                type_inference::Status::Solved => (),
                type_inference::Status::Inapplicable(conflict) => {
                    log::debug!(
                        "inapplicable type inference: {diagnostic}",
                        diagnostic = conflict.diagnostic(&conditions, &self.prog.files)
                    );
                    report.failed_type_inference += 1;
                    continue;
                }
                type_inference::Status::Underconstrained => {
                    let underconstrained = solution.assignment.underconstrained();
                    let mut diagnostic = format!(
                        "underconstrained type inference: could not infer a concrete type for \
                         {n} expression(s) in expansion '{description}'. The following \
                         expressions have ambiguous types; add type annotations or term \
                         signatures to constrain them:",
                        n = underconstrained.len(),
                    );
                    for x in underconstrained {
                        let tv = solution
                            .assignment
                            .assignment(x)
                            .expect("underconstrained expression must have a type value");
                        let position = conditions
                            .pos
                            .get(&x)
                            .map(|pos| pos.pretty_print_line(&self.prog.files))
                            .unwrap_or_else(|| "?".to_string());
                        let expr = &conditions.exprs[x.index()];
                        diagnostic.push_str(&format!(
                            "\n\t{position}: e{x} = {expr} (inferred type: {tv})",
                            x = x.index(),
                        ));
                    }
                    bail!(diagnostic)
                }
                type_inference::Status::TypeError(confict) => {
                    return Err(conditions.error_at_expr(
                        &self.prog,
                        confict.x,
                        confict.reason.clone(),
                    ));
                }
            }

            // Verify.
            if self.skip_solver {
                log::debug!("Skipping solver");
                continue;
            }

            let solution_log_dir = log_dir.join(format!("{:03}", i));
            let verify_report = self
                .verify_expansion_type_instantiation(
                    &conditions,
                    &solution.assignment,
                    solver_backend,
                    solution_log_dir,
                    &mut output,
                    expansion,
                    id,
                    &description,
                    i,
                    failures,
                )
                .context(format!("verify expansion: {id}"))?;

            // Append to report.
            let duration = start_solution.elapsed();
            report.type_instantiations.push(TypeInstantationReport {
                choices,
                verify: verify_report,
                duration,
            });
        }

        // End timer
        report.duration = start.elapsed();

        Ok(report)
    }

    fn select_solver_backend(&self, expansion: &Expansion) -> Result<SolverBackend> {
        for solver_rule in &self.solver_rules {
            if self.eval_predicate(&solver_rule.predicate, expansion)? {
                return Ok(solver_rule.solver_backend);
            }
        }
        Ok(self.default_solver_backend)
    }

    fn verify_expansion_type_instantiation(
        &self,
        conditions: &Conditions,
        assignment: &Assignment,
        solver_backend: SolverBackend,
        log_dir: std::path::PathBuf,
        output: &mut dyn Write,
        expansion: &Expansion,
        expansion_id: usize,
        description: &str,
        instantiation_index: usize,
        failures: &Mutex<Vec<FailureRecord>>,
    ) -> Result<VerifyReport> {
        let start = time::Instant::now();

        // Solve.
        let binary = solver_backend.prog();
        let args = solver_backend.args(self.timeout);
        let replay_file = Self::open_log_file(log_dir.clone(), "solver.smt2")?;
        let smt = easy_smt::ContextBuilder::new()
            .solver(binary, &args)
            .replay_file(Some(replay_file))
            .build()?;

        let mut solver = Solver::new(smt, &self.prog, conditions, assignment)?;
        solver.set_dialect(solver_backend.dialect());
        solver.encode()?;
        let init_time = start.elapsed();

        // Applicability check.
        let start = time::Instant::now();
        let applicability = solver.check_assumptions_feasibility()?;
        let applicable_time = start.elapsed();

        writeln!(output, "\t\tapplicability = {applicability}")?;
        match applicability {
            Applicability::Applicable => (),
            Applicability::Inapplicable => {
                return Ok(VerifyReport {
                    verdict: Verdict::Inapplicable,
                    init_time,
                    applicable_time,
                    verify_time: None,
                });
            }
            Applicability::Unknown => {
                let unknown_path = log_dir.join("applicability_unknown.out");
                let mut unknown_file =
                    Self::open_log_file(log_dir.clone(), "applicability_unknown.out")?;
                writeln!(
                    unknown_file,
                    "#{expansion_id}\t{description}\tinstantiation={instantiation_index}"
                )?;
                writeln!(unknown_file, "expansion:")?;
                write_expansion(&mut unknown_file, &self.prog, expansion)?;

                writeln!(
                    output,
                    "\t\tapplicability unknown, written to {}",
                    unknown_path.display()
                )?;
                log::warn!(
                    "applicability unknown: #{expansion_id} {description} (expansion written to {})",
                    unknown_path.display()
                );

                failures.lock().unwrap().push(FailureRecord {
                    kind: FailureKind::ApplicabilityUnknown,
                    expansion_id,
                    description: description.to_string(),
                    instantiation_index,
                    failure_path: unknown_path,
                });

                return Ok(VerifyReport {
                    verdict: Verdict::ApplicabilityUnknown,
                    init_time,
                    applicable_time,
                    verify_time: None,
                });
            }
        };

        // Verify.
        let start = time::Instant::now();
        let verification = solver.check_verification_condition()?;
        let verify_time = Some(start.elapsed());

        writeln!(output, "\t\tverification = {verification}")?;
        Ok(match verification {
            Verification::Failure(model) => {
                let failure_path = log_dir.join("failure.out");
                let mut failure_file = Self::open_log_file(log_dir.clone(), "failure.out")?;
                writeln!(
                    failure_file,
                    "#{expansion_id}\t{description}\tinstantiation={instantiation_index}"
                )?;
                writeln!(failure_file, "expansion:")?;
                write_expansion(&mut failure_file, &self.prog, expansion)?;
                writeln!(failure_file, "model:")?;
                conditions.write_model(&mut failure_file, &model, &self.prog)?;

                writeln!(output, "\t\tfailure written to {}", failure_path.display())?;
                log::warn!(
                    "verification failure: #{expansion_id} {description} (model written to {})",
                    failure_path.display()
                );

                failures.lock().unwrap().push(FailureRecord {
                    kind: FailureKind::Verification,
                    expansion_id,
                    description: description.to_string(),
                    instantiation_index,
                    failure_path,
                });

                VerifyReport {
                    verdict: Verdict::Failure,
                    init_time,
                    applicable_time,
                    verify_time,
                }
            }
            Verification::Success => VerifyReport {
                verdict: Verdict::Success,
                init_time,
                applicable_time,
                verify_time,
            },
            Verification::Unknown => VerifyReport {
                verdict: Verdict::Unknown,
                init_time,
                applicable_time,
                verify_time,
            },
        })
    }

    fn open_log_file<P: AsRef<Path>>(log_dir: std::path::PathBuf, name: P) -> Result<File> {
        std::fs::create_dir_all(&log_dir)?;
        let path = log_dir.join(name);
        let file = File::create(&path)?;
        Ok(file)
    }
}

/// Human-readable description of an expansion.
fn expansion_description(expansion: &Expansion, prog: &Program) -> Result<String> {
    let rule_id = expansion
        .rules
        .first()
        .ok_or(format_err!("expansion should have at least one rule"))?;
    let rule = prog.rule(*rule_id);
    Ok(rule.identifier(&prog.tyenv, &prog.files))
}
