use anyhow::{Context, Result};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use structopt::StructOpt;

/// Convert Souper optimizations into Peepmatic DSL.
#[derive(StructOpt)]
pub struct Options {
    /// Specify an input file to be used. Use '-' for stdin.
    #[structopt(parse(from_os_str))]
    input: PathBuf,

    /// Specify the output file to be used. Use '-' for stdout.
    #[structopt(short("o"), long("output"), default_value("-"), parse(from_os_str))]
    output: PathBuf,
}

pub fn run(options: &Options) -> Result<()> {
    let peepmatic_dsl = if options.input == Path::new("-") {
        let stdin = std::io::stdin();
        let mut stdin = stdin.lock();
        let mut souper_dsl = vec![];
        stdin
            .read_to_end(&mut souper_dsl)
            .context("failed to read from stdin")?;
        let souper_dsl = String::from_utf8(souper_dsl).context("stdin is not UTF-8: {}")?;
        peepmatic_souper::convert_str(&souper_dsl, Some(Path::new("stdin")))?
    } else {
        peepmatic_souper::convert_file(&options.input)?
    };

    if options.output == Path::new("-") {
        let stdout = std::io::stdout();
        let mut stdout = stdout.lock();
        stdout
            .write_all(peepmatic_dsl.as_bytes())
            .context("error writing to stdout")?;
    } else {
        std::fs::write(&options.output, peepmatic_dsl.as_bytes())
            .with_context(|| format!("error writing to {}", options.output.display()))?;
    }

    Ok(())
}
