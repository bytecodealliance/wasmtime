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
pub fn generate_rust_assembler<P: AsRef<Path>>(dir: P, file: &str) -> PathBuf {
    generate(dir, file, generate::rust_assembler, Language::Rust)
}

/// Generate a macro containing builder functions for the assembler's ISLE
/// constructors; this uses the `build` module emitted in
/// [`generate_rust_assembler`].
pub fn generate_isle_macro<P: AsRef<Path>>(dir: P, file: &str) -> PathBuf {
    generate(dir, file, generate::isle_macro, Language::Rust)
}

/// Generate the ISLE definitions; this provides ISLE glue to access the builder
/// functions from [`generate_isle_macro`].
pub fn generate_isle_definitions<P: AsRef<Path>>(dir: P, file: &str) -> PathBuf {
    generate(dir, file, generate::isle_definitions, Language::Isle)
}

/// Helper for emitting generated lines into a formatted file.
///
/// # Panics
///
/// This function panics if we cannot update the file.
fn generate<P: AsRef<Path>>(
    dir: P,
    file: &str,
    generator: fn(&mut Formatter, &[dsl::Inst]),
    lang: Language,
) -> PathBuf {
    let out = dir.as_ref().join(file);
    eprintln!("Generating {}", out.display());
    let mut fmt = Formatter::new(lang);
    generator(&mut fmt, &instructions::list());
    fmt.write(file, dir.as_ref()).unwrap();
    if matches!(lang, Language::Rust) {
        rustfmt(&out);
    }
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
