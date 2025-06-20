use clap::Parser;
use cranelift_isle::compile;
use cranelift_isle::error::Errors;
use std::{
    fs,
    io::{self, Write},
    path::PathBuf,
};

#[derive(Parser)]
struct Opts {
    /// The output file to write the generated Rust code to. `stdout` is used if
    /// this is not given.
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// The input ISLE DSL source files.
    #[arg(required = true)]
    inputs: Vec<PathBuf>,
}

fn main() -> Result<(), Errors> {
    let _ = env_logger::try_init();

    let opts = Opts::parse();
    let code = compile::from_files(opts.inputs, &Default::default())?;

    let stdout = io::stdout();
    let (mut output, output_name): (Box<dyn Write>, _) = match &opts.output {
        Some(f) => {
            let output =
                Box::new(fs::File::create(f).map_err(|e| {
                    Errors::from_io(e, format!("failed to create '{}'", f.display()))
                })?);
            (output, f.display().to_string())
        }
        None => {
            let output = Box::new(stdout.lock());
            (output, "<stdout>".to_string())
        }
    };

    output
        .write_all(code.as_bytes())
        .map_err(|e| Errors::from_io(e, format!("failed to write to '{output_name}'")))?;

    Ok(())
}
