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

    set_commit_info_for_rustc();

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
            test_directory_module(out, "tests/misc_testsuite/gc", strategy)?;
            // The testsuite of Winch is a subset of the official
            // WebAssembly test suite, until parity is reached. This
            // check is in place to prevent Cranelift from duplicating
            // tests.
            if *strategy == "Winch" {
                test_directory_module(out, "tests/misc_testsuite/winch", strategy)?;
            }
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
                test_directory_module(out, "tests/spec_testsuite/proposals/gc", strategy)?;
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

    // Ignore some tests for when testing Winch.
    if strategy == "Winch" {
        if testsuite == "misc_testsuite" {
            let denylist = [
                "externref_id_function",
                "int_to_float_splat",
                "issue6562",
                "many_table_gets_lead_to_gc",
                "mutable_externref_globals",
                "no_mixup_stack_maps",
                "no_panic",
                "simple_ref_is_null",
                "table_grow_with_funcref",
            ];
            return denylist.contains(&testname);
        }
        if testsuite == "spec_testsuite" {
            let denylist = [
                "br_table",
                "global",
                "table_fill",
                "table_get",
                "table_set",
                "table_grow",
                "table_size",
                "elem",
                "select",
                "unreached_invalid",
                "linking",
            ]
            .contains(&testname);

            let ref_types = testname.starts_with("ref_");
            let simd = testname.starts_with("simd_");

            return denylist || ref_types || simd;
        }

        if testsuite == "memory64" {
            return testname.starts_with("simd") || testname.starts_with("threads");
        }

        if testsuite != "winch" {
            return true;
        }
    }

    // This is an empty file right now which the `wast` crate doesn't parse
    if testname.contains("memory_copy1") {
        return true;
    }

    if testsuite == "gc" {
        if [
            "array_copy",
            "array_fill",
            "array_init_data",
            "array_init_elem",
            "array",
            "binary_gc",
            "binary",
            "br_on_cast_fail",
            "br_on_cast",
            "br_on_non_null",
            "br_on_null",
            "br_table",
            "call_ref",
            "data",
            "elem",
            "extern",
            "func",
            "global",
            "if",
            "linking",
            "local_get",
            "local_init",
            "ref_as_non_null",
            "ref_cast",
            "ref_eq",
            "ref_is_null",
            "ref_null",
            "ref_test",
            "ref",
            "return_call_indirect",
            "return_call_ref",
            "return_call",
            "select",
            "struct",
            "table_sub",
            "table",
            "type_canon",
            "type_equivalence",
            "type_rec",
            "type_subtyping",
            "unreached_invalid",
            "unreached_valid",
        ]
        .contains(&testname)
        {
            return true;
        }
    }

    match env::var("CARGO_CFG_TARGET_ARCH").unwrap().as_str() {
        "s390x" => {
            // TODO(#6530): These tests require tail calls, but s390x
            // doesn't support them yet.
            testsuite == "function_references" || testsuite == "tail_call"
        }

        _ => false,
    }
}

fn set_commit_info_for_rustc() {
    if !Path::new(".git").exists() {
        return;
    }
    let output = match Command::new("git")
        .arg("log")
        .arg("-1")
        .arg("--date=short")
        .arg("--format=%H %h %cd")
        .arg("--abbrev=9")
        .output()
    {
        Ok(output) if output.status.success() => output,
        _ => return,
    };
    let stdout = String::from_utf8(output.stdout).unwrap();
    let mut parts = stdout.split_whitespace();
    let mut next = || parts.next().unwrap();
    println!("cargo:rustc-env=WASMTIME_GIT_HASH={}", next());
    println!(
        "cargo:rustc-env=WASMTIME_VERSION_INFO={} ({} {})",
        env!("CARGO_PKG_VERSION"),
        next(),
        next()
    );
}
