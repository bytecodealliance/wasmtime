//! Build program to generate a program which runs all the testsuites.
//!
//! By generating a separate `#[test]` test for each file, we allow cargo test
//! to automatically run the files in parallel.
//!
//! Idea adapted from: https://github.com/CraneStation/wasmtime/blob/master/build.rs
//! Thanks @sunfishcode

use std::env;
use std::fs::{read_dir, DirEntry, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn main() {
    let out_dir =
        PathBuf::from(env::var("OUT_DIR").expect("The OUT_DIR environment variable must be set"));
    println!("OUT_DIR is {:?}", out_dir);
    let mut out = File::create(out_dir.join("misc_testsuite_tests.rs"))
        .expect("error generating test source file");

    build_tests("misc_testsuite", &out_dir).expect("building tests");
    test_directory(&mut out, "misc_testsuite", &out_dir).expect("generating tests");
}

fn build_tests(testsuite: &str, out_dir: &Path) -> io::Result<()> {
    // if the submodule has not been checked out, the build will stall
    if !Path::new(&format!("{}/Cargo.toml", testsuite)).exists() {
        panic!("Testsuite {} not checked out", testsuite);
    }

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
        "    fn {}() -> Result<(), String> {{",
        avoid_keywords(&stemstr.replace("-", "_"))
    )?;
    writeln!(out, "        setup_log();")?;
    write!(out, "        let path = std::path::Path::new(\"")?;
    // Write out the string with escape_debug to prevent special characters such
    // as backslash from being reinterpreted.
    for c in path.display().to_string().chars() {
        write!(out, "{}", c.escape_debug())?;
    }
    writeln!(out, "\");")?;
    writeln!(out, "        let data = utils::read_wasm(path)?;")?;
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

cfg_if::cfg_if! {
    if #[cfg(not(windows))] {
        /// Ignore tests that aren't supported yet.
        fn ignore(testsuite: &str, name: &str) -> bool {
            if testsuite == "misc_testsuite" {
                match name {
                    "path_symlink_trailing_slashes" => true,
                    _ => false,
                }
            } else {
                unreachable!()
            }
        }
    } else {
        /// Ignore tests that aren't supported yet.
        fn ignore(testsuite: &str, name: &str) -> bool {
            if testsuite == "misc_testsuite" {
                match name {
                    "readlink_no_buffer" => true,
                    "dangling_symlink" => true,
                    "symlink_loop" => true,
                    "clock_time_get" => true,
                    "truncation_rights" => true,
                    "fd_readdir" => true,
                    "path_rename_trailing_slashes" => true,
                    "path_symlink_trailing_slashes" => true,
                    "remove_directory_trailing_slashes" => true,
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
    if testsuite == "misc_testsuite" {
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
