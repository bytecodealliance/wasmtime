//! Utility functions.

use anyhow::Context;
use std::fs::File;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Read an entire file into a string.
pub fn read_to_string<P: AsRef<Path>>(path: P) -> anyhow::Result<String> {
    let mut buffer = String::new();
    let path = path.as_ref();
    if path == Path::new("-") {
        let stdin = io::stdin();
        let mut stdin = stdin.lock();
        stdin
            .read_to_string(&mut buffer)
            .context("failed to read stdin to string")?;
    } else {
        let mut file = File::open(path)?;
        file.read_to_string(&mut buffer)
            .with_context(|| format!("failed to read {} to string", path.display()))?;
    }
    Ok(buffer)
}

/// Iterate over all of the files passed as arguments, recursively iterating through directories.
pub fn iterate_files<'a>(files: &'a [PathBuf]) -> impl Iterator<Item = PathBuf> + 'a {
    files
        .iter()
        .flat_map(WalkDir::new)
        .filter(|f| match f {
            Ok(d) => {
                // Filter out hidden files (starting with .).
                !d.file_name().to_str().map_or(false, |s| s.starts_with('.'))
                    // Filter out directories.
                    && !d.file_type().is_dir()
            }
            Err(e) => {
                println!("Unable to read file: {e}");
                false
            }
        })
        .map(|f| {
            f.expect("this should not happen: we have already filtered out the errors")
                .into_path()
        })
}
