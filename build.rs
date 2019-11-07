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
                &to_os_path(&["spec_testsuite", "proposals", "simd", "simd_address.wast"]),
                strategy,
            )
            .expect("generating tests");
            test_file(
                &mut out,
                &to_os_path(&["spec_testsuite", "proposals", "simd", "simd_align.wast"]),
                strategy,
            )
            .expect("generating tests");
            test_file(
                &mut out,
                &to_os_path(&["spec_testsuite", "proposals", "simd", "simd_const.wast"]),
                strategy,
            )
            .expect("generating tests");

            let multi_value_suite = &to_os_path(&["spec_testsuite", "proposals", "multi-value"]);
            test_directory(&mut out, &multi_value_suite, strategy).expect("generating tests");
        } else {
            println!("cargo:warning=The spec testsuite is disabled. To enable, run `git submodule update --remote`.");
        }

        writeln!(out, "}}").expect("generating tests");
    }
}

/// Helper for creating OS-independent paths.
fn to_os_path(components: &[&str]) -> String {
    let path: PathBuf = components.iter().collect();
    path.display().to_string()
}

fn test_directory(out: &mut File, path: &str, strategy: &str) -> io::Result<()> {
    let mut dir_entries: Vec<_> = read_dir(path)
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

    let testsuite = &extract_name(path);
    start_test_module(out, testsuite)?;
    for dir_entry in dir_entries {
        write_testsuite_tests(out, &dir_entry.path(), testsuite, strategy)?;
    }
    finish_test_module(out)
}

fn test_file(out: &mut File, testfile: &str, strategy: &str) -> io::Result<()> {
    let path = Path::new(testfile);
    let testsuite = format!("single_test_{}", extract_name(path));
    start_test_module(out, &testsuite)?;
    write_testsuite_tests(out, path, &testsuite, strategy)?;
    finish_test_module(out)
}

/// Extract a valid Rust identifier from the stem of a path.
fn extract_name(path: impl AsRef<Path>) -> String {
    path.as_ref()
        .file_stem()
        .expect("filename should have a stem")
        .to_str()
        .expect("filename should be representable as a string")
        .replace("-", "_")
        .replace("/", "_")
}

fn start_test_module(out: &mut File, testsuite: &str) -> io::Result<()> {
    writeln!(out, "    mod {} {{", testsuite)?;
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
    let testname = extract_name(path);

    writeln!(out, "        #[test]")?;
    if ignore(testsuite, &testname, strategy) {
        writeln!(out, "        #[ignore]")?;
    }
    writeln!(out, "        fn r#{}() {{", &testname)?;
    writeln!(out, "            let isa = native_isa();")?;
    writeln!(
        out,
        "            let compiler = Compiler::new(isa, CompilationStrategy::{});",
        strategy
    )?;
    writeln!(
        out,
        "            let features = Features {{ simd: {}, multi_value: {}, ..Default::default() }};",
        testsuite.contains("simd"),
        testsuite.contains("multi_value")
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
fn ignore(testsuite: &str, testname: &str, strategy: &str) -> bool {
    let is_multi_value = testsuite.ends_with("multi_value");
    match strategy {
        #[cfg(feature = "lightbeam")]
        "Lightbeam" => match (testsuite, testname) {
            (_, _) if testname.starts_with("simd") => return true,
            (_, _) if is_multi_value => return true,
            _ => (),
        },
        "Cranelift" => match (testsuite, testname) {
            // We don't currently support more return values than available
            // registers, and this contains a function with many, many more
            // return values than that.
            (_, "func") if is_multi_value => return true,
            _ => {}
        },
        _ => panic!("unrecognized strategy"),
    }

    if cfg!(windows) {
        return match (testsuite, testname) {
            // Currently, our multi-value support only works with however many
            // extra return registers we have available, and windows' fastcall
            // ABI only has a single return register, so we need to wait on full
            // multi-value support in Cranelift.
            (_, _) if is_multi_value => true,
            (_, _) => false,
        };
    }

    false
}
