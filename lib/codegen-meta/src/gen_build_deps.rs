//! Generate build dependencies for Cargo.
//!
//! The `build.rs` script is invoked by cargo when building lib/codegen to
//! generate Rust code from the instruction descriptions. Cargo needs to know when
//! it is necessary to rerun the build script.
//!
//! If the build script outputs lines of the form:
//!     cargo:rerun-if-changed=/path/to/file
//!
//! cargo will rerun the build script when those files have changed since the last
//! build.

use error;

use std::fs;
use std::path;

/// Recursively find all interesting source files and directories in the
/// directory tree starting at `dir`. Yield a path to each file.
fn source_files(dir: &path::PathBuf) -> Result<Vec<String>, error::Error> {
    let mut files = Vec::new();
    if dir.is_dir() {
        for entry in fs::read_dir(&dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                let mut child_dir_files = source_files(&path)?;
                files.append(&mut child_dir_files);
            } else if let Some(ext) = path.extension() {
                if ext == "rs" {
                    files.push(path.to_str().unwrap().to_string());
                }
            }
        }
    }
    Ok(files)
}

/// Generate the lines of `cargo:rerun-if-changed` output, for each Rust source
/// file inside of the cranelift-codegen-meta crate.
pub fn generate(meta_dir: &path::PathBuf) -> Result<(), error::Error> {
    println!("Dependencies from Rust meta language directory:");
    source_files(&meta_dir)?
        .into_iter()
        .for_each(|p| println!("cargo:rerun-if-changed={}", p));

    Ok(())
}
