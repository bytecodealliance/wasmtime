//! Build program to generate a program which runs all the testsuites.
//!
//! By generating a separate `#[test]` test for each file, we allow cargo test
//! to automatically run the files in parallel.

use anyhow::Context;
use std::env;
use std::fmt::Write;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() -> anyhow::Result<()> {
    println!("cargo:rerun-if-changed=build.rs");

    let out_dir = PathBuf::from(
        env::var_os("OUT_DIR").expect("The OUT_DIR environment variable must be set"),
    );
    let mut out = String::new();

    for strategy in &["Cranelift", "Winch"] {
        writeln!(out, "#[cfg(test)]")?;
        writeln!(out, "#[allow(non_snake_case)]")?;
        if *strategy == "Winch" {
            // We only test Winch on x86_64, for now.
            writeln!(out, "{}", "#[cfg(all(target_arch = \"x86_64\"))]")?;
        }
        writeln!(out, "mod {} {{", strategy)?;

        with_test_module(&mut out, "misc", |out| {
            test_directory(out, "tests/misc_testsuite", strategy)?;
            test_directory_module(out, "tests/misc_testsuite/multi-memory", strategy)?;
            test_directory_module(out, "tests/misc_testsuite/simd", strategy)?;
            test_directory_module(out, "tests/misc_testsuite/tail-call", strategy)?;
            test_directory_module(out, "tests/misc_testsuite/threads", strategy)?;
            test_directory_module(out, "tests/misc_testsuite/memory64", strategy)?;
            test_directory_module(out, "tests/misc_testsuite/component-model", strategy)?;
            test_directory_module(out, "tests/misc_testsuite/function-references", strategy)?;
            test_directory_module(out, "tests/misc_testsuite/winch", strategy)?;
            Ok(())
        })?;

        with_test_module(&mut out, "spec", |out| {
            let spec_tests = test_directory(out, "tests/spec_testsuite", strategy)?;
            // Skip running spec_testsuite tests if the submodule isn't checked
            // out.
            if spec_tests > 0 {
                test_directory_module(out, "tests/spec_testsuite/proposals/memory64", strategy)?;
                test_directory_module(
                    out,
                    "tests/spec_testsuite/proposals/function-references",
                    strategy,
                )?;
                test_directory_module(
                    out,
                    "tests/spec_testsuite/proposals/multi-memory",
                    strategy,
                )?;
                test_directory_module(out, "tests/spec_testsuite/proposals/threads", strategy)?;
                test_directory_module(
                    out,
                    "tests/spec_testsuite/proposals/relaxed-simd",
                    strategy,
                )?;
                test_directory_module(out, "tests/spec_testsuite/proposals/tail-call", strategy)?;
            } else {
                println!(
                    "cargo:warning=The spec testsuite is disabled. To enable, run `git submodule \
                     update --remote`."
                );
            }
            Ok(())
        })?;

        writeln!(out, "}}")?;
    }

    // Write out our auto-generated tests and opportunistically format them with
    // `rustfmt` if it's installed.
    let output = out_dir.join("wast_testsuite_tests.rs");
    fs::write(&output, out)?;
    drop(Command::new("rustfmt").arg(&output).status());
    Ok(())
}

fn test_directory_module(
    out: &mut String,
    path: impl AsRef<Path>,
    strategy: &str,
) -> anyhow::Result<usize> {
    let path = path.as_ref();
    let testsuite = &extract_name(path);
    with_test_module(out, testsuite, |out| test_directory(out, path, strategy))
}

fn test_directory(
    out: &mut String,
    path: impl AsRef<Path>,
    strategy: &str,
) -> anyhow::Result<usize> {
    let path = path.as_ref();
    let mut dir_entries: Vec<_> = path
        .read_dir()
        .context(format!("failed to read {:?}", path))?
        .map(|r| r.expect("reading testsuite directory entry"))
        .filter_map(|dir_entry| {
            let p = dir_entry.path();
            let ext = p.extension()?;
            // Only look at wast files.
            if ext != "wast" {
                return None;
            }
            // Ignore files starting with `.`, which could be editor temporary files
            if p.file_stem()?.to_str()?.starts_with('.') {
                return None;
            }
            Some(p)
        })
        .collect();

    dir_entries.sort();

    let testsuite = &extract_name(path);
    for entry in dir_entries.iter() {
        write_testsuite_tests(out, entry, testsuite, strategy, false)?;
        write_testsuite_tests(out, entry, testsuite, strategy, true)?;
    }

    Ok(dir_entries.len())
}

/// Extract a valid Rust identifier from the stem of a path.
fn extract_name(path: impl AsRef<Path>) -> String {
    path.as_ref()
        .file_stem()
        .expect("filename should have a stem")
        .to_str()
        .expect("filename should be representable as a string")
        .replace(['-', '/'], "_")
}

fn with_test_module<T>(
    out: &mut String,
    testsuite: &str,
    f: impl FnOnce(&mut String) -> anyhow::Result<T>,
) -> anyhow::Result<T> {
    out.push_str("mod ");
    out.push_str(testsuite);
    out.push_str(" {\n");

    let result = f(out)?;

    out.push_str("}\n");
    Ok(result)
}

fn write_testsuite_tests(
    out: &mut String,
    path: impl AsRef<Path>,
    testsuite: &str,
    strategy: &str,
    pooling: bool,
) -> anyhow::Result<()> {
    let path = path.as_ref();
    let testname = extract_name(path);

    writeln!(out, "#[test]")?;
    // Ignore when using QEMU for running tests (limited memory).
    if ignore(testsuite, &testname, strategy) {
        writeln!(out, "#[ignore]")?;
    } else {
        writeln!(out, "#[cfg_attr(miri, ignore)]")?;
    }

    writeln!(
        out,
        "fn r#{}{}() {{",
        &testname,
        if pooling { "_pooling" } else { "" }
    )?;
    writeln!(out, "    let _ = env_logger::try_init();")?;
    writeln!(
        out,
        "    crate::wast::run_wast(r#\"{}\"#, crate::wast::Strategy::{}, {}).unwrap();",
        path.display(),
        strategy,
        pooling,
    )?;
    writeln!(out, "}}")?;
    writeln!(out)?;
    Ok(())
}

/// Ignore tests that aren't supported yet.
fn ignore(testsuite: &str, testname: &str, strategy: &str) -> bool {
    assert!(strategy == "Cranelift" || strategy == "Winch");

    // Ignore everything except the winch misc test suite.
    // We ignore tests that assert for traps on windows, given
    // that Winch doesn't encode unwind information for Windows, yet.
    if strategy == "Winch" {
        if testsuite != "winch" {
            return true;
        }

        let assert_trap = ["i32", "i64"].contains(&testname);

        if assert_trap && env::var("CARGO_CFG_TARGET_OS").unwrap().as_str() == "windows" {
            return true;
        }
    }

    // This is an empty file right now which the `wast` crate doesn't parse
    if testname.contains("memory_copy1") {
        return true;
    }

    if testsuite == "function_references" {
        // The following tests fail due to function references not yet
        // being exposed in the public API.
        if testname == "ref_null" || testname == "local_init" {
            return true;
        }
        // This test fails due to incomplete support for the various
        // table/elem syntactic sugar in wasm-tools/wast.
        if testname == "br_table" {
            return true;
        }
        // This test fails due to the current implementation of type
        // canonicalisation being broken as a result of
        // #[derive(hash)] on WasmHeapType.
        if testname == "type_equivalence" {
            return true;
        }
    }

    match env::var("CARGO_CFG_TARGET_ARCH").unwrap().as_str() {
        "s390x" => {
            // FIXME: These tests fail under qemu due to a qemu bug.
            testname == "simd_f32x4_pmin_pmax" || testname == "simd_f64x2_pmin_pmax"
                // TODO(#6530): These tests require tail calls, but s390x
                // doesn't support them yet.
                || testsuite == "function_references" || testsuite == "tail_call"
        }

        "riscv64" => {
            if testsuite.contains("relaxed_simd") {
                return true;
            }

            let known_failure = [
                "canonicalize_nan",
                "cvt_from_uint",
                "issue_3327_bnot_lowering",
                "simd_conversions",
                "simd_f32x4_rounding",
                "simd_f64x2_rounding",
                "simd_i32x4_trunc_sat_f32x4",
                "simd_i32x4_trunc_sat_f64x2",
                "simd_load",
                "simd_splat",
            ]
            .contains(&testname);

            known_failure
        }

        _ => false,
    }
}
