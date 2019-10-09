//! Build program to generate a program which runs all the testsuites.
//!
//! By generating a separate `#[test]` test for each file, we allow cargo test
//! to automatically run the files in parallel.

use std::env;
use std::fs::{read_dir, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

fn main() {
    let out_dir =
        PathBuf::from(env::var("OUT_DIR").expect("The OUT_DIR environment variable must be set"));
    let mut out = File::create(out_dir.join("wast_testsuite_tests.rs"))
        .expect("error generating test source file");

    for strategy in &[
        "Cranelift",
        #[cfg(feature = "lightbeam")]
        "Lightbeam",
    ] {
        writeln!(out, "#[cfg(test)]").expect("generating tests");
        writeln!(out, "#[allow(non_snake_case)]").expect("generating tests");
        writeln!(out, "mod {} {{", strategy).expect("generating tests");

        test_directory(&mut out, "misc_testsuite", strategy).expect("generating tests");
        test_directory(&mut out, "spec_testsuite", strategy).expect("generating tests");
        // Skip running spec_testsuite tests if the submodule isn't checked out.
        if read_dir("spec_testsuite")
            .expect("reading testsuite directory")
            .next()
            .is_some()
        {
            test_file(
                &mut out,
                "spec_testsuite/proposals/simd/simd_const.wast",
                strategy,
            )
            .expect("generating tests");
        } else {
            println!("cargo:warning=The spec testsuite is disabled. To enable, run `git submodule update --remote`.");
        }

        writeln!(out, "}}").expect("generating tests");
    }
}

fn test_directory(out: &mut File, testsuite: &str, strategy: &str) -> io::Result<()> {
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

    start_test_module(out, testsuite)?;
    for dir_entry in dir_entries {
        write_testsuite_tests(out, &dir_entry.path(), testsuite, strategy)?;
    }
    finish_test_module(out)
}

fn test_file(out: &mut File, testfile: &str, strategy: &str) -> io::Result<()> {
    let testsuite = "single_file_spec_test";
    let path = Path::new(testfile);
    start_test_module(out, testsuite)?;
    write_testsuite_tests(out, path, testsuite, strategy)?;
    finish_test_module(out)
}

fn start_test_module(out: &mut File, testsuite: &str) -> io::Result<()> {
    writeln!(
        out,
        "    mod {} {{",
        Path::new(testsuite)
            .file_stem()
            .expect("testsuite filename should have a stem")
            .to_str()
            .expect("testsuite filename should be representable as a string")
            .replace("-", "_"),
    )?;
    writeln!(
        out,
        "        use super::super::{{native_isa, Path, WastContext, Compiler, Features, CompilationStrategy}};"
    )
}

fn finish_test_module(out: &mut File) -> io::Result<()> {
    writeln!(out, "    }}")
}

fn write_testsuite_tests(
    out: &mut File,
    path: &Path,
    testsuite: &str,
    strategy: &str,
) -> io::Result<()> {
    let stemstr = path
        .file_stem()
        .expect("file_stem")
        .to_str()
        .expect("to_str");

    writeln!(out, "        #[test]")?;
    if ignore(testsuite, stemstr, strategy) {
        writeln!(out, "        #[ignore]")?;
    }
    writeln!(out, "        fn r#{}() {{", &stemstr.replace("-", "_"))?;
    writeln!(out, "            let isa = native_isa();")?;
    writeln!(
        out,
        "            let compiler = Compiler::new(isa, CompilationStrategy::{});",
        strategy
    )?;
    writeln!(
        out,
        "            let features = Features {{ simd: true, ..Default::default() }};"
    )?;
    writeln!(
        out,
        "            let mut wast_context = WastContext::new(Box::new(compiler)).with_features(features);"
    )?;
    writeln!(out, "            wast_context")?;
    writeln!(out, "                .register_spectest()")?;
    writeln!(
        out,
        "                .expect(\"instantiating \\\"spectest\\\"\");"
    )?;
    writeln!(out, "            wast_context")?;
    write!(out, "                .run_file(Path::new(\"")?;
    // Write out the string with escape_debug to prevent special characters such
    // as backslash from being reinterpreted.
    for c in path.display().to_string().chars() {
        write!(out, "{}", c.escape_debug())?;
    }
    writeln!(out, "\"))")?;
    writeln!(out, "                .expect(\"error running wast file\");",)?;
    writeln!(out, "        }}")?;
    writeln!(out)?;
    Ok(())
}

/// Ignore tests that aren't supported yet.
fn ignore(testsuite: &str, name: &str, strategy: &str) -> bool {
    match strategy {
        #[cfg(feature = "lightbeam")]
        "Lightbeam" => match (testsuite, name) {
            ("single_file_spec_test", "simd_const") => return true,
            _ => (),
        },
        "Cranelift" => {}
        _ => panic!("unrecognized strategy"),
    }

    if cfg!(windows) {
        return match (testsuite, name) {
            ("spec_testsuite", "address") => true,
            ("spec_testsuite", "align") => true,
            ("spec_testsuite", "call") => true,
            ("spec_testsuite", "call_indirect") => true,
            ("spec_testsuite", "conversions") => true,
            ("spec_testsuite", "elem") => true,
            ("spec_testsuite", "fac") => true,
            ("spec_testsuite", "func_ptrs") => true,
            ("spec_testsuite", "globals") => true,
            ("spec_testsuite", "i32") => true,
            ("spec_testsuite", "i64") => true,
            ("spec_testsuite", "f32") => true,
            ("spec_testsuite", "f64") => true,
            ("spec_testsuite", "if") => true,
            ("spec_testsuite", "imports") => true,
            ("spec_testsuite", "int_exprs") => true,
            ("spec_testsuite", "linking") => true,
            ("spec_testsuite", "memory_grow") => true,
            ("spec_testsuite", "memory_trap") => true,
            ("spec_testsuite", "resizing") => true,
            ("spec_testsuite", "select") => true,
            ("spec_testsuite", "skip-stack-guard-page") => true,
            ("spec_testsuite", "start") => true,
            ("spec_testsuite", "traps") => true,
            ("spec_testsuite", "unreachable") => true,
            ("spec_testsuite", "unwind") => true,
            ("misc_testsuite", "misc_traps") => true,
            ("misc_testsuite", "stack_overflow") => true,
            (_, _) => false,
        };
    }

    #[cfg(target_os = "linux")]
    {
        // Test whether the libc correctly parses the following constant; if so,
        // we can run the "const" test. If not, the "const" test will fail, since
        // we use wabt to parse the tests and wabt uses strtof.
        extern "C" {
            pub fn strtof(s: *const libc::c_char, endp: *mut *mut libc::c_char) -> libc::c_float;
        }
        if unsafe {
            strtof(
                b"8.8817847263968443574e-16" as *const u8 as *const libc::c_char,
                core::ptr::null_mut(),
            )
        }
        .to_bits()
            != 0x26800001
        {
            return match (testsuite, name) {
                ("spec_testsuite", "const") => true,
                ("single_file_spec_test", "simd_const") => true,
                (_, _) => false,
            };
        }
    }

    false
}
