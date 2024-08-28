//! Prototype verification tool for Cranelift's ISLE lowering rules.

use clap::{ArgAction, Parser};
use cranelift_codegen_meta::{generate_isle, isle::get_isle_compilations};
use std::path::PathBuf;
use std::{env, fs};
use veri_engine_lib::verify::verify_rules;
use veri_engine_lib::Config;

#[derive(Parser)]
#[clap(about, version, author)]
struct Args {
    /// Sets the input file
    #[clap(short, long)]
    input: Option<String>,

    /// Which LHS root to verify
    #[clap(short, long, default_value = "lower")]
    term: String,

    /// Which width types to verify
    #[clap(long)]
    widths: Option<Vec<String>>,

    /// Which named rule to verify
    #[clap(long)]
    names: Option<Vec<String>>,

    /// Don't use the prelude ISLE files
    #[clap(short, long, action=ArgAction::SetTrue)]
    noprelude: bool,

    /// Include the aarch64 files
    #[clap(short, long, action=ArgAction::SetTrue)]
    aarch64: bool,

    /// Include the x64 files
    #[clap(short, long, action=ArgAction::SetTrue)]
    x64: bool,

    /// Don't check for distinct possible models
    #[clap(long, action=ArgAction::SetTrue)]
    nodistinct: bool,
}

impl Args {
    fn isle_input_files(&self) -> anyhow::Result<Vec<std::path::PathBuf>> {
        // Generate ISLE files.
        let cur_dir = env::current_dir().expect("Can't access current working directory");
        let gen_dir = cur_dir.join("output");
        if !std::path::Path::new(gen_dir.as_path()).exists() {
            fs::create_dir_all(gen_dir.as_path()).unwrap();
        }
        generate_isle(gen_dir.as_path()).expect("Can't generate ISLE");

        let codegen_crate_dir = cur_dir.join("../../../codegen");
        let inst_specs_isle = codegen_crate_dir.join("src").join("inst_specs.isle");

        // Lookup ISLE compilations.
        let compilations = get_isle_compilations(codegen_crate_dir.as_path(), gen_dir.as_path());

        let name = match (self.aarch64, self.x64) {
            (true, false) => "aarch64",
            (false, true) => "x64",
            _ => panic!("aarch64 of x64 backend must be provided"),
        };

        let mut inputs = compilations
            .lookup(name)
            .ok_or(anyhow::format_err!("unknown ISLE compilation: {}", name))?
            .inputs();
        inputs.push(inst_specs_isle);

        // Return inputs from the matching compilation, if any.
        Ok(inputs)
    }
}

fn main() -> anyhow::Result<()> {
    env_logger::init();
    let args = Args::parse();

    let valid_widths = ["I8", "I16", "I32", "I64"];
    if let Some(widths) = &args.widths {
        for w in widths {
            let w_str = w.as_str();
            if !valid_widths.contains(&w_str) {
                panic!("Invalid width type: {}", w);
            }
        }
    }

    let inputs = if args.noprelude {
        vec![PathBuf::from(
            args.input.expect("Missing input file in noprelude mode"),
        )]
    } else {
        args.isle_input_files()?
    };

    let names = if let Some(names) = args.names {
        let mut names = names;
        names.sort();
        names.dedup();
        Some(names)
    } else {
        None
    };

    let config = Config {
        term: args.term,
        names: names,
        distinct_check: !args.nodistinct,
        custom_verification_condition: None,
        custom_assumptions: None,
    };

    verify_rules(inputs, &config, &args.widths)
        .map_err(|e| anyhow::anyhow!("failed to compile ISLE: {:?}", e))
}
