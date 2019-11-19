//! Build program to generate a program which runs all the testsuites.
//!
//! By generating a separate `#[test]` test for each file, we allow cargo test
//! to automatically run the files in parallel.

use std::env;
use std::fs::{read_dir, DirEntry, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn main() {
    build_and_generate_tests()
}

fn build_and_generate_tests() {
    // Validate if any of test sources are present and if they changed
    // This should always work since there is no submodule to init anymore
    let bin_tests = std::fs::read_dir("wasi-tests/src/bin").unwrap();
    for test in bin_tests {
        if let Ok(test_file) = test {
            let test_file_path = test_file
                .path()
                .into_os_string()
                .into_string()
                .expect("test file path");
            println!("cargo:rerun-if-changed={}", test_file_path);
        }
    }
    // Build tests to OUT_DIR (target/*/build/wasi-common-*/out/wasm32-wasi/release/*.wasm)
    let out_dir =
        PathBuf::from(env::var("OUT_DIR").expect("The OUT_DIR environment variable must be set"));
    let mut out =
        File::create(out_dir.join("wasi_tests.rs")).expect("error generating test source file");
    build_tests("wasi-tests", &out_dir).expect("building tests");
    test_directory(&mut out, "wasi-tests", &out_dir).expect("generating tests");
}

fn build_tests(testsuite: &str, out_dir: &Path) -> io::Result<()> {
    let mut cmd = Command::new("cargo");
    cmd.args(&[
        "build",
        "--release",
        "--target=wasm32-wasi",
        "--target-dir",
        out_dir.to_str().unwrap(),
    ])
    .stdout(Stdio::inherit())
    .stderr(Stdio::inherit())
    .current_dir(testsuite);
    let output = cmd.output()?;

    let status = output.status;
    if !status.success() {
        panic!(
            "Building tests failed: exit code: {}",
            status.code().unwrap()
        );
    }

    Ok(())
}

fn test_directory(out: &mut File, testsuite: &str, out_dir: &Path) -> io::Result<()> {
    let mut dir_entries: Vec<_> = read_dir(out_dir.join("wasm32-wasi/release"))
        .expect("reading testsuite directory")
        .map(|r| r.expect("reading testsuite directory entry"))
        .filter(|dir_entry| {
            let p = dir_entry.path();
            if let Some(ext) = p.extension() {
                // Only look at wast files.
                if ext == "wasm" {
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
    writeln!(out, "    use super::{{runtime, utils, setup_log}};")?;
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
        "    fn r#{}() -> anyhow::Result<()> {{",
        &stemstr.replace("-", "_")
    )?;
    writeln!(out, "        setup_log();")?;
    writeln!(
        out,
        "        let path = std::path::Path::new(r#\"{}\"#);",
        path.display()
    )?;
    writeln!(out, "        let data = wat::parse_file(path)?;")?;
    writeln!(
        out,
        "        let bin_name = utils::extract_exec_name_from_path(path)?;"
    )?;
    let workspace = if no_preopens(testsuite, stemstr) {
        "None"
    } else {
        writeln!(
            out,
            "        let workspace = utils::prepare_workspace(&bin_name)?;"
        )?;
        "Some(workspace.path())"
    };
    writeln!(
        out,
        "        runtime::instantiate(&data, &bin_name, {})",
        workspace
    )?;
    writeln!(out, "    }}")?;
    writeln!(out)?;
    Ok(())
}

cfg_if::cfg_if! {
    if #[cfg(not(windows))] {
        /// Ignore tests that aren't supported yet.
        fn ignore(_testsuite: &str, _name: &str) -> bool {
            false
        }
    } else {
        /// Ignore tests that aren't supported yet.
        fn ignore(testsuite: &str, name: &str) -> bool {
            if testsuite == "wasi-tests" {
                match name {
                    "readlink_no_buffer" => true,
                    "dangling_symlink" => true,
                    "symlink_loop" => true,
                    "truncation_rights" => true,
                    "poll_oneoff" => true,
                    "path_link" => true,
                    "dangling_fd" => true,
                    _ => false,
                }
            } else {
                unreachable!()
            }
        }
    }
}

/// Mark tests which do not require preopens
fn no_preopens(testsuite: &str, name: &str) -> bool {
    if testsuite == "wasi-tests" {
        match name {
            "big_random_buf" => true,
            "clock_time_get" => true,
            "sched_yield" => true,
            _ => false,
        }
    } else {
        unreachable!()
    }
}
