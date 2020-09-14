use std::io::{Read, Write};
use std::path::Path;

pub fn run(input: &Path, output: &Path) -> Result<(), String> {
    let peepmatic_dsl = if input == Path::new("-") {
        let stdin = std::io::stdin();
        let mut stdin = stdin.lock();
        let mut souper_dsl = vec![];
        stdin
            .read_to_end(&mut souper_dsl)
            .map_err(|e| format!("failed to read from stdin: {}", e))?;
        let souper_dsl =
            String::from_utf8(souper_dsl).map_err(|e| format!("stdin is not UTF-8: {}", e))?;
        peepmatic_souper::convert_str(&souper_dsl, Some(Path::new("stdin")))
            .map_err(|e| e.to_string())?
    } else {
        peepmatic_souper::convert_file(input).map_err(|e| e.to_string())?
    };

    if output == Path::new("-") {
        let stdout = std::io::stdout();
        let mut stdout = stdout.lock();
        stdout
            .write_all(peepmatic_dsl.as_bytes())
            .map_err(|e| format!("error writing to stdout: {}", e))?;
    } else {
        std::fs::write(output, peepmatic_dsl.as_bytes())
            .map_err(|e| format!("error writing to {}: {}", output.display(), e))?;
    }

    Ok(())
}
