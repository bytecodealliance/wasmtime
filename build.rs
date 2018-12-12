//! Build program to generate a program which runs all the testsuites.
//!
//! By generating a separate `#[test]` test for each file, we allow cargo test
//! to automatically run the files in parallel.

use std::env;
use std::fs::{read_dir, DirEntry, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

fn main() {
    let out_dir =
        PathBuf::from(env::var("OUT_DIR").expect("The OUT_DIR environment variable must be set"));
    let mut out = File::create(out_dir.join("wast_testsuite_tests.rs"))
        .expect("error generating test source file");

    test_directory(&mut out, "misc_testsuite").expect("generating tests");
    test_directory(&mut out, "spec_testsuite").expect("generating tests");
}

fn test_directory(out: &mut File, testsuite: &str) -> io::Result<()> {
    let mut dir_entries: Vec<_> = read_dir(testsuite)
        .expect("reading testsuite directory")
        .map(|r| r.expect("reading testsuite directory entry"))
        .filter(|dir_entry| {
            let p = dir_entry.path();
            if let Some(ext) = p.extension() {
                // Only look at wast files.
                if ext == "wast" {
                    // Ignore files starting with `.`, which could be editor temporary files
                    if let Some(stem) = p.file_stem() {
                        if let Some(stemstr) = stem.to_str() {
                            if !stemstr.starts_with('.') {
                                return true;
                            }
                        }
                    }
                }
            }
            false
        })
        .collect();

    dir_entries.sort_by_key(|dir| dir.path());

    writeln!(
        out,
        "mod {} {{",
        Path::new(testsuite)
            .file_stem()
            .expect("testsuite filename should have a stem")
            .to_str()
            .expect("testsuite filename should be representable as a string")
            .replace("-", "_")
    )?;
    writeln!(out, "    use super::{{native_isa, Path, WastContext}};")?;
    for dir_entry in dir_entries {
        write_testsuite_tests(out, dir_entry, testsuite)?;
    }
    writeln!(out, "}}")?;
    Ok(())
}

fn write_testsuite_tests(out: &mut File, dir_entry: DirEntry, testsuite: &str) -> io::Result<()> {
    let path = dir_entry.path();
    let stemstr = path
        .file_stem()
        .expect("file_stem")
        .to_str()
        .expect("to_str");

    writeln!(out, "    #[test]")?;
    if ignore(testsuite, stemstr) {
        writeln!(out, "    #[ignore]")?;
    }
    writeln!(
        out,
        "    fn {}() {{",
        avoid_keywords(&stemstr.replace("-", "_"))
    )?;
    writeln!(out, "        let mut wast_context = WastContext::new();")?;
    writeln!(out, "        let isa = native_isa();")?;
    writeln!(out, "        wast_context")?;
    writeln!(out, "            .register_spectest()")?;
    writeln!(
        out,
        "            .expect(\"instantiating \\\"spectest\\\"\");"
    )?;
    writeln!(out, "        wast_context")?;
    writeln!(
        out,
        "            .run_file(&*isa, Path::new(\"{}\"))",
        path.display()
    )?;
    writeln!(out, "            .expect(\"error running wast file\");",)?;
    writeln!(out, "    }}")?;
    writeln!(out)?;
    Ok(())
}

/// Rename tests which have the same name as Rust keywords.
fn avoid_keywords(name: &str) -> &str {
    match name {
        "if" => "if_",
        "loop" => "loop_",
        "type" => "type_",
        "const" => "const_",
        "return" => "return_",
        other => other,
    }
}

/// Ignore tests that aren't supported yet.
fn ignore(_testsuite: &str, _name: &str) -> bool {
    false
}
