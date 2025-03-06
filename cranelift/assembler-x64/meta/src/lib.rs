//! This crate generates Cranelift-specific assembly code for x64 instructions; see the `README.md`
//! for more information.

pub mod dsl;
mod generate;
pub mod instructions;

use cranelift_srcgen::{Formatter, Language};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Generate the assembler `file` containing the core assembler logic; each of
/// the DSL-defined instructions is emitted into a Rust `enum Inst`.
///
/// # Panics
///
/// This function panics if we cannot update the file.
pub fn generate_rust_assembler<P: AsRef<Path>>(dir: P, file: &str) -> PathBuf {
    let out = dir.as_ref().join(file);
    eprintln!("Generating {}", out.display());
    let mut fmt = Formatter::new(Language::Rust);
    generate::rust_assembler(&mut fmt, &instructions::list());
    fmt.write(file, dir.as_ref()).unwrap();
    rustfmt(&out);
    out
}

/// Use the installed `rustfmt` binary to format the generated code; if it
/// fails, skip formatting with a warning.
fn rustfmt(file: &Path) {
    if let Ok(status) = Command::new("rustfmt").arg(file).status() {
        if !status.success() {
            eprintln!("`rustfmt` exited with a non-zero status; skipping formatting of generated files");
        }
    } else {
        eprintln!("`rustfmt` not found; skipping formatting of generated files");
    }
}
