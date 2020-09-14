use anyhow::{Context, Result};
use std::io::{Read, Write};
use std::path::Path;

pub fn run(input: &Path, output: &Path) -> Result<()> {
    let peepmatic_dsl = if input == Path::new("-") {
        let stdin = std::io::stdin();
        let mut stdin = stdin.lock();
        let mut souper_dsl = vec![];
        stdin
            .read_to_end(&mut souper_dsl)
            .context("failed to read from stdin")?;
        let souper_dsl = String::from_utf8(souper_dsl).context("stdin is not UTF-8: {}")?;
        peepmatic_souper::convert_str(&souper_dsl, Some(Path::new("stdin")))?
    } else {
        peepmatic_souper::convert_file(input)?
    };

    if output == Path::new("-") {
        let stdout = std::io::stdout();
        let mut stdout = stdout.lock();
        stdout
            .write_all(peepmatic_dsl.as_bytes())
            .context("error writing to stdout")?;
    } else {
        std::fs::write(output, peepmatic_dsl.as_bytes())
            .with_context(|| format!("error writing to {}", output.display()))?;
    }

    Ok(())
}
