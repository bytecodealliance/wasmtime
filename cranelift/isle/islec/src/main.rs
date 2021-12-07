use cranelift_isle::{compile, lexer, parser};
use miette::{Context, IntoDiagnostic, Result};
use std::{
    fs,
    io::{self, Write},
    path::PathBuf,
};
use structopt::StructOpt;

#[derive(StructOpt)]
struct Opts {
    /// The output file to write the generated Rust code to. `stdout` is used if
    /// this is not given.
    #[structopt(short, long, parse(from_os_str))]
    output: Option<PathBuf>,

    /// The input ISLE DSL source files.
    #[structopt(parse(from_os_str))]
    inputs: Vec<PathBuf>,
}

fn main() -> Result<()> {
    let _ = env_logger::try_init();

    let _ = miette::set_hook(Box::new(|_| {
        Box::new(
            miette::MietteHandlerOpts::new()
                // `miette` mistakenly uses braille-optimized output for emacs's
                // `M-x shell`.
                .force_graphical(true)
                .build(),
        )
    }));

    let opts = Opts::from_args();

    let lexer = lexer::Lexer::from_files(opts.inputs)?;
    let defs = parser::parse(lexer)?;
    let code = compile::compile(&defs)?;

    let stdout = io::stdout();
    let (mut output, output_name): (Box<dyn Write>, _) = match &opts.output {
        Some(f) => {
            let output = Box::new(
                fs::File::create(f)
                    .into_diagnostic()
                    .with_context(|| format!("failed to create '{}'", f.display()))?,
            );
            (output, f.display().to_string())
        }
        None => {
            let output = Box::new(stdout.lock());
            (output, "<stdout>".to_string())
        }
    };

    output
        .write_all(code.as_bytes())
        .into_diagnostic()
        .with_context(|| format!("failed to write to '{}'", output_name))?;

    Ok(())
}
