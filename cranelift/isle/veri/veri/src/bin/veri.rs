use std::path::Path;
use std::time::Duration;

use anyhow::{Context as _, Result, format_err};
use clap::{ArgAction, Parser};
use cranelift_codegen_meta::{generate_isle, isle::get_isle_compilations};
use cranelift_isle_veri::runner::{Filter, Runner, SolverBackend, SolverRule};

/// Configuration file applied by default when no `--config` is given.
const DEFAULT_CONFIG: &str = "cranelift/isle/veri/configs/aarch64-fast.args";

#[derive(Parser)]
// Allow a single-valued argument to appear more than once, with the last
// occurrence winning. This lets a value set in a `--config` file  be overridden
// on the command line. Multi-valued arguments such as `--filter` still accumulate.
#[command(args_override_self = true)]
struct Opts {
    /// Read additional arguments from one or more configuration files.
    ///
    /// Each file lists command-line arguments, one or more per line (blank
    /// lines and `#` comments are ignored). The arguments are applied before
    /// those given directly on the command line, so anything passed on the
    /// command line overrides the configuration.
    #[arg(long = "config", value_name = "FILE")]
    config: Vec<std::path::PathBuf>,

    /// Name of the ISLE compilation.
    #[arg(long, default_value = "aarch64")]
    name: String,

    /// Path to codegen crate directory.
    #[arg(long, default_value = "cranelift/codegen")]
    codegen_crate_dir: std::path::PathBuf,

    /// Working directory. Defaults to a fresh temporary directory.
    #[arg(long)]
    work_dir: Option<std::path::PathBuf>,

    /// Filter expansions.
    #[arg(long = "filter", value_name = "FILTER")]
    filters: Vec<Filter>,

    /// Exclude a default set of tags that are not yet well supported:
    /// `vector`, `atomics`, `spectre`, `narrowfloat`, `amode_const`, and `i128`.
    #[arg(long)]
    default_excludes: bool,

    /// Only expand from the given root term, instead of all terms with rules.
    #[arg(long = "only-root", value_name = "TERM")]
    only_root: Option<String>,

    /// Verify only the expansions containing the named rule.
    ///
    /// Expansion is seeded from the rule's root term (so the rule is reached
    /// even if that term has no standalone spec), then narrowed to the
    /// expansions that actually contain the rule. Conflicts with `--only-root`.
    #[arg(long = "rule", value_name = "RULE", conflicts_with = "only_root")]
    rule: Option<String>,

    /// Don't skip expansions tagged TODO.
    #[arg(long = "no-skip-todo", action = ArgAction::SetFalse)]
    skip_todo: bool,

    /// Solver backend to use.
    #[arg(long = "solver", default_value = "cvc5", env = "ISLE_VERI_SOLVER")]
    solver_backend: SolverBackend,

    /// Solver selection rule of the form `<solver>=<predicate>`. Earlier rules take precedence.
    #[arg(long = "solver-rule")]
    solver_rules: Vec<SolverRule>,

    /// Ignore explicit solver selection tags `solver_<solver>`.
    #[arg(long)]
    ignore_solver_tags: bool,

    /// Per-query timeout, in seconds.
    #[arg(long, default_value = "30", env = "ISLE_VERI_TIMEOUT")]
    timeout: u64,

    /// Number of threads to use (0 defaults to # of logical cores)
    #[arg(long, default_value = "0")]
    num_threads: usize,

    /// Log directory.
    #[arg(long)]
    log_dir: Option<std::path::PathBuf>,

    /// Write results to files under log directory. (Use 0 to select automatically.)
    #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
    results_to_log_dir: bool,

    /// Skip solver.
    #[arg(long, env = "ISLE_VERI_SKIP_SOLVER")]
    skip_solver: bool,

    /// Dump debug output.
    #[arg(long)]
    debug: bool,
}

impl Opts {
    fn isle_input_files(&self, work_dir: &std::path::Path) -> Result<Vec<std::path::PathBuf>> {
        // Generate ISLE files.
        let gen_dir = work_dir;
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

/// Collect the paths passed via `--config` directly from the raw arguments.
fn collect_config_paths(args: &[String]) -> Vec<std::path::PathBuf> {
    let mut paths = Vec::new();
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        if let Some(path) = arg.strip_prefix("--config=") {
            paths.push(std::path::PathBuf::from(path));
        } else if arg == "--config"
            && let Some(path) = args.next()
        {
            paths.push(std::path::PathBuf::from(path));
        }
    }
    paths
}

/// Read a configuration file into a list of arguments. Each line may contain
/// one or more whitespace-separated arguments. Blank lines are ignored, as is
/// any text following a `#`
fn read_config_args(path: &Path) -> Result<Vec<String>> {
    let contents = std::fs::read_to_string(path)
        .with_context(|| format!("reading config file {}", path.display()))?;
    let mut args = Vec::new();
    for line in contents.lines() {
        // Strip trailing (or whole-line) comment.
        let line = match line.split_once('#') {
            Some((before, _)) => before,
            None => line,
        };
        for token in line.split_whitespace() {
            args.push(token.to_string());
        }
    }
    Ok(args)
}

fn main() -> Result<()> {
    env_logger::builder().format_target(false).init();

    // Expand any `--config` files into the argument list before parsing.
    // Configuration arguments are placed ahead of the arguments given on the
    // command line so that the command line takes precedence.
    let raw: Vec<String> = std::env::args().collect();
    let mut config_paths = collect_config_paths(&raw[1..]);
    if config_paths.is_empty() {
        config_paths.push(std::path::PathBuf::from(DEFAULT_CONFIG));
    }
    let opts = if config_paths.is_empty() {
        Opts::parse()
    } else {
        let mut combined = vec![raw[0].clone()];
        for path in &config_paths {
            combined.extend(read_config_args(path)?);
        }
        combined.extend(raw[1..].iter().cloned());
        Opts::parse_from(combined)
    };

    // Setup thread pool.
    //
    // Recursively evaluating complex spec encodings (e.g., `clz`) can overflow
    // the default Rayon worker thread stack size, so default to a larger stack.
    const DEFAULT_STACK_SIZE: usize = 256 * 1024 * 1024;
    let stack_size = std::env::var("ISLE_VERI_STACK_SIZE")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_STACK_SIZE);
    rayon::ThreadPoolBuilder::new()
        .num_threads(opts.num_threads)
        .stack_size(stack_size)
        .build_global()?;
    log::info!("num theads: {}", rayon::current_num_threads());

    // Resolve the working directory, defaulting to a fresh temporary directory
    // that lives for the duration of the run.
    let temp_dir = match &opts.work_dir {
        Some(_) => None,
        None => Some(tempfile::tempdir()?),
    };
    let work_dir: &std::path::Path = match (&opts.work_dir, &temp_dir) {
        (Some(dir), _) => dir,
        (None, Some(temp)) => temp.path(),
        (None, None) => unreachable!(),
    };

    // Read ISLE inputs.
    let inputs = opts.isle_input_files(work_dir)?;
    let mut runner = Runner::from_files(&inputs)?;

    // Scope expansion to a single root term, if requested. Otherwise the
    // default is to expand from every term that has rules.
    if let Some(root) = &opts.only_root {
        runner.set_root_term(root);
    }

    // Scope verification to a single named rule, if requested. This seeds
    // expansion from the rule's root term and filters down to the expansions
    // containing the rule.
    if let Some(rule) = &opts.rule {
        runner.set_root_rule(rule)?;
    }

    // Configure runner.
    // Default behaviour is to include every expansion (all paths from all
    // roots); any provided filters only narrow that down via `exclude`.
    let mut filters = opts.filters.clone();
    let default_exclude_tags: &[&str] = &[
        "vector",
        "atomics",
        "spectre",
        "narrowfloat",
        "amode_const",
        "i128",
        "slow",
        "wasm_category_stack",
    ];
    if opts.default_excludes {
        for tag in default_exclude_tags {
            filters.push(format!("exclude:tag:{tag}").parse()?);
        }
    }
    runner.filters(&filters);
    if opts.skip_todo {
        runner.skip_tag("TODO");
    }

    runner.set_default_solver_backend(opts.solver_backend);
    if !opts.ignore_solver_tags {
        runner.add_solver_tag_rules();
    }
    for solver_rule in opts.solver_rules {
        runner.add_solver_rule(solver_rule);
    }

    runner.set_timeout(Duration::from_secs(opts.timeout));
    // Effective log directory: the runner defaults to `.veriisle` unless
    // overridden here.
    let log_dir = opts
        .log_dir
        .clone()
        .unwrap_or_else(|| std::path::PathBuf::from(".veriisle"));
    if let Some(log_dir) = opts.log_dir {
        runner.set_log_dir(log_dir);
    }
    runner.set_results_to_log_dir(opts.results_to_log_dir);
    runner.skip_solver(opts.skip_solver);
    runner.debug(opts.debug);

    // Summarize what is being excluded and where output is going before
    // starting verification.
    println!("========================== Verification configuration =========================");
    println!("Number of threads:  {}", rayon::current_num_threads());
    println!("Working directory:  {}", work_dir.display());
    println!("Log directory:      {}", log_dir.display());
    println!(
        "Results to log dir: {}",
        if opts.results_to_log_dir {
            format!("yes (results.out under {})", log_dir.display())
        } else {
            "no (results printed to stdout)".to_string()
        }
    );
    if let Some(root) = &opts.only_root {
        println!("only root term:     {root}");
    }
    if let Some(rule) = &opts.rule {
        println!("only rule:          {rule}");
    }
    if opts.default_excludes {
        println!(
            "Excluding ISLE terms with any of the following tags (the default exclude set): {}.",
            default_exclude_tags.join(", ")
        );
    } else {
        println!(
            "Not applying any default tag exclusions (pass --default-excludes to skip the tags that are not yet well supported)."
        );
    }
    if opts.skip_todo {
        println!("Excluding ISLE terms tagged TODO.");
    } else {
        println!("Including ISLE terms tagged TODO.");
    }
    if opts.filters.is_empty() {
        println!("Not applying any additional filters.");
    } else {
        println!("Also applying the following filters:");
        for filter in &opts.filters {
            println!("  - {}", filter.describe());
        }
    }
    println!("==========================");

    runner.run()?;

    Ok(())
}
