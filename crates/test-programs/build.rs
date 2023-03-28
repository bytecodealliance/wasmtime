#![allow(dead_code)]
//! Build program to generate a program which runs all the testsuites.
//!
//! By generating a separate `#[test]` test for each file, we allow cargo test
//! to automatically run the files in parallel.
use std::fs::{read_dir, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn main() {
    #[cfg(feature = "test_programs")]
    wasi_tests::build_and_generate_tests();
    #[cfg(feature = "test_programs_http")]
    wasi_http_tests::build_and_generate_tests();
}

fn build_tests(testsuite: &str, out_dir: &Path) -> io::Result<Vec<String>> {
    let mut cmd = Command::new("cargo");
    cmd.env("CARGO_PROFILE_RELEASE_DEBUG", "1");
    cmd.env_remove("CARGO_ENCODED_RUSTFLAGS");
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

    let meta = cargo_metadata::MetadataCommand::new()
        .manifest_path(PathBuf::from(testsuite).join("Cargo.toml"))
        .exec()
        .expect("cargo metadata");

    Ok(meta
        .packages
        .iter()
        .find(|p| p.name == testsuite)
        .unwrap()
        .targets
        .iter()
        .filter(|t| t.kind == ["bin"])
        .map(|t| t.name.clone())
        .collect::<Vec<String>>())
}

#[allow(dead_code)]
fn test_directory(
    out: &mut File,
    test_binaries: &[String],
    testsuite: &str,
    runtime: &str,
    out_dir: &Path,
    mut write_testsuite_tests: impl FnMut(&mut File, &Path, &str) -> io::Result<()>,
) -> io::Result<()> {
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
    writeln!(
        out,
        "    use super::{{runtime::{} as runtime, utils, setup_log}};",
        runtime
    )?;
    for test_binary in test_binaries {
        let binary_path = out_dir
            .join("wasm32-wasi")
            .join("release")
            .join(format!("{}.wasm", test_binary.replace("-", "_")));
        write_testsuite_tests(out, &binary_path, testsuite)?;
    }
    writeln!(out, "}}")?;
    Ok(())
}

#[cfg(feature = "test_programs")]
mod wasi_tests {
    use super::*;
    use std::env;

    pub(super) fn build_and_generate_tests() {
        // Validate if any of test sources are present and if they changed
        // This should always work since there is no submodule to init anymore
        let bin_tests = read_dir("wasi-tests/src/bin").unwrap();
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
        println!("cargo:rerun-if-changed=wasi-tests/Cargo.toml");
        println!("cargo:rerun-if-changed=wasi-tests/src/lib.rs");
        // Build tests to OUT_DIR (target/*/build/wasi-common-*/out/wasm32-wasi/release/*.wasm)
        let out_dir = PathBuf::from(
            env::var("OUT_DIR").expect("The OUT_DIR environment variable must be set"),
        );
        let mut out =
            File::create(out_dir.join("wasi_tests.rs")).expect("error generating test source file");
        let test_binaries = build_tests("wasi-tests", &out_dir).expect("building tests");
        test_directory(
            &mut out,
            &test_binaries,
            "wasi-cap-std-sync",
            "cap_std_sync",
            &out_dir,
            write_testsuite_tests,
        )
        .expect("generating wasi-cap-std-sync tests");
        test_directory(
            &mut out,
            &test_binaries,
            "wasi-tokio",
            "tokio",
            &out_dir,
            write_testsuite_tests,
        )
        .expect("generating wasi-tokio tests");
    }

    fn write_testsuite_tests(out: &mut File, path: &Path, testsuite: &str) -> io::Result<()> {
        let stemstr = path
            .file_stem()
            .expect("file_stem")
            .to_str()
            .expect("to_str");

        writeln!(out, "    #[test]")?;
        let test_fn_name = stemstr.replace("-", "_");
        if ignore(testsuite, &test_fn_name) {
            writeln!(out, "    #[ignore]")?;
        }
        writeln!(out, "    fn r#{}() -> anyhow::Result<()> {{", test_fn_name,)?;
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
            "        runtime::{}(&data, &bin_name, {})",
            if inherit_stdio(testsuite, stemstr) {
                "instantiate_inherit_stdio"
            } else {
                "instantiate"
            },
            workspace,
        )?;
        writeln!(out, "    }}")?;
        writeln!(out)?;
        Ok(())
    }

    fn ignore(testsuite: &str, name: &str) -> bool {
        match testsuite {
            "wasi-cap-std-sync" => cap_std_sync_ignore(name),
            "wasi-virtfs" => virtfs_ignore(name),
            "wasi-tokio" => tokio_ignore(name),
            _ => panic!("unknown test suite: {}", testsuite),
        }
    }

    #[cfg(not(windows))]
    /// Ignore tests that aren't supported yet.
    fn cap_std_sync_ignore(name: &str) -> bool {
        [
            // Trailing slash related bugs:
            "path_rename_file_trailing_slashes",
            "remove_directory_trailing_slashes",
        ]
        .contains(&name)
    }

    #[cfg(windows)]
    /// Ignore tests that aren't supported yet.
    fn cap_std_sync_ignore(name: &str) -> bool {
        [
            // Trailing slash related bugs
            "interesting_paths",
            "path_rename_file_trailing_slashes",
            "remove_directory_trailing_slashes",
        ]
        .contains(&name)
    }

    /// Tokio should support the same things as cap_std_sync
    fn tokio_ignore(name: &str) -> bool {
        cap_std_sync_ignore(name)
    }
    /// Virtfs barely works at all and is not suitable for any purpose
    fn virtfs_ignore(name: &str) -> bool {
        [
            "dangling_fd",
            "dangling_symlink",
            "directory_seek",
            "fd_advise",
            "fd_filestat_set",
            "fd_flags_set",
            "fd_readdir",
            "file_allocate",
            "file_pread_pwrite",
            "file_seek_tell",
            "file_truncation",
            "file_unbuffered_write",
            "interesting_paths",
            "isatty",
            "nofollow_errors",
            "path_filestat",
            "path_link",
            "path_open_create_existing",
            "path_open_dirfd_not_dir",
            "path_open_read_without_rights",
            "path_rename",
            "path_rename_dir_trailing_slashes",
            "path_rename_file_trailing_slashes",
            "path_symlink_trailing_slashes",
            "poll_oneoff",
            "poll_oneoff_stdio",
            "readlink",
            "remove_directory_trailing_slashes",
            "remove_nonempty_directory",
            "renumber",
            "symlink_create",
            "symlink_filestat",
            "symlink_loop",
            "truncation_rights",
            "unlink_file_trailing_slashes",
        ]
        .contains(&name)
    }

    /// Mark tests which do not require preopens
    fn no_preopens(testsuite: &str, name: &str) -> bool {
        if testsuite.starts_with("wasi-") {
            match name {
                "big_random_buf" => true,
                "clock_time_get" => true,
                "sched_yield" => true,
                "poll_oneoff_stdio" => true,
                _ => false,
            }
        } else {
            panic!("unknown test suite {}", testsuite)
        }
    }

    /// Mark tests which require inheriting parent process stdio
    fn inherit_stdio(testsuite: &str, name: &str) -> bool {
        match testsuite {
            "wasi-cap-std-sync" | "wasi-tokio" => match name {
                "poll_oneoff_stdio" => true,
                _ => false,
            },
            "wasi-virtfs" => false,
            _ => panic!("unknown test suite {}", testsuite),
        }
    }
}

#[cfg(feature = "test_programs_http")]
mod wasi_http_tests {
    use super::*;
    use std::env;

    pub(super) fn build_and_generate_tests() {
        // Validate if any of test sources are present and if they changed
        // This should always work since there is no submodule to init anymore
        let bin_tests = read_dir("wasi-http-tests/src/bin").unwrap();
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
        println!("cargo:rerun-if-changed=wasi-http-tests/Cargo.toml");
        println!("cargo:rerun-if-changed=wasi-http-tests/src/lib.rs");
        // Build tests to OUT_DIR (target/*/build/wasi-common-*/out/wasm32-wasi/release/*.wasm)
        let out_dir = PathBuf::from(
            env::var("OUT_DIR").expect("The OUT_DIR environment variable must be set"),
        );
        let mut out = File::create(out_dir.join("wasi_http_tests.rs"))
            .expect("error generating test source file");

        let test_binaries = build_tests("wasi-http-tests", &out_dir).expect("building tests");
        test_directory(
            &mut out,
            &test_binaries,
            "wasi-http-tests",
            "wasi_http_tests",
            &out_dir,
            write_testsuite_tests,
        )
        .expect("generating wasi-cap-std-sync tests");
    }

    fn write_testsuite_tests(out: &mut File, path: &Path, _testsuite: &str) -> io::Result<()> {
        let stemstr = path
            .file_stem()
            .expect("file_stem")
            .to_str()
            .expect("to_str");

        writeln!(out, "    #[test]")?;
        let test_fn_name = stemstr.replace("-", "_");
        writeln!(out, "    fn r#{}() -> anyhow::Result<()> {{", test_fn_name,)?;
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
        writeln!(
            out,
            "        runtime::instantiate_inherit_stdio(&data, &bin_name, None)",
        )?;
        writeln!(out, "    }}")?;
        writeln!(out)?;
        Ok(())
    }
}
