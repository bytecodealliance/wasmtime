//! Run the tests in `wasi_testsuite` using Wasmtime's CLI binary and checking
//! the results with a [wasi-testsuite] spec.
//!
//! [wasi-testsuite]: https://github.com/WebAssembly/wasi-testsuite

use anyhow::{Result, anyhow};
use libtest_mimic::{Arguments, Trial};
use serde_derive::Deserialize;
use std::collections::HashMap;
use std::fmt::Write;
use std::fs;
use std::path::Path;
use std::process::Output;
use tempfile::TempDir;
use wit_component::ComponentEncoder;

const KNOWN_FAILURES: &[&str] = &[
    "filesystem-hard-links",
    "filesystem-read-directory",
    // FIXME(#11524)
    "remove_directory_trailing_slashes",
    #[cfg(target_vendor = "apple")]
    "filesystem-advise",
    // FIXME(WebAssembly/wasi-testsuite#128)
    #[cfg(windows)]
    "fd_fdstat_set_rights",
    #[cfg(windows)]
    "filesystem-flags-and-type",
    #[cfg(windows)]
    "path_link",
    #[cfg(windows)]
    "dangling_fd",
    #[cfg(windows)]
    "dangling_symlink",
    #[cfg(windows)]
    "file_allocate",
    #[cfg(windows)]
    "file_pread_pwrite",
    #[cfg(windows)]
    "file_seek_tell",
    #[cfg(windows)]
    "file_truncation",
    #[cfg(windows)]
    "file_unbuffered_write",
    #[cfg(windows)]
    "interesting_paths",
    #[cfg(windows)]
    "isatty",
    #[cfg(windows)]
    "fd_readdir",
    #[cfg(windows)]
    "nofollow_errors",
    #[cfg(windows)]
    "overwrite_preopen",
    #[cfg(windows)]
    "path_exists",
    #[cfg(windows)]
    "path_filestat",
    #[cfg(windows)]
    "path_open_create_existing",
    #[cfg(windows)]
    "path_open_dirfd_not_dir",
    #[cfg(windows)]
    "path_open_missing",
    #[cfg(windows)]
    "path_open_preopen",
    #[cfg(windows)]
    "path_open_read_write",
    #[cfg(windows)]
    "path_rename",
    #[cfg(windows)]
    "path_rename_dir_trailing_slashes",
    #[cfg(windows)]
    "path_symlink_trailing_slashes",
    #[cfg(windows)]
    "readlink",
    #[cfg(windows)]
    "remove_nonempty_directory",
    #[cfg(windows)]
    "renumber",
    #[cfg(windows)]
    "symlink_create",
    #[cfg(windows)]
    "stdio",
    #[cfg(windows)]
    "symlink_filestat",
    #[cfg(windows)]
    "truncation_rights",
    #[cfg(windows)]
    "symlink_loop",
    #[cfg(windows)]
    "unlink_file_trailing_slashes",
    // Once cm-async changes have percolated this can be removed.
    "filesystem-flags-and-type",
    "multi-clock-wait",
    "monotonic-clock",
    "filesystem-advise",
];

fn main() -> Result<()> {
    env_logger::init();

    let mut trials = Vec::new();
    if !cfg!(miri) {
        find_tests("tests/wasi_testsuite/wasi-common".as_ref(), &mut trials).unwrap();
        find_tests("tests/wasi_testsuite/wasi-threads".as_ref(), &mut trials).unwrap();
    }

    libtest_mimic::run(&Arguments::from_args(), trials).exit()
}

fn find_tests(path: &Path, trials: &mut Vec<Trial>) -> Result<()> {
    for entry in path.read_dir()? {
        let entry = entry?;
        let path = entry.path();
        if entry.file_type()?.is_dir() {
            find_tests(&path, trials)?;
            continue;
        }
        if path.extension().and_then(|s| s.to_str()) != Some("wasm") {
            continue;
        }

        // Test the core wasm itself.
        trials.push(Trial::test(
            format!("wasmtime-wasi - {}", path.display()),
            {
                let path = path.clone();
                move || run_test(&path, false).map_err(|e| format!("{e:?}").into())
            },
        ));

        // Also test the component version using the wasip1 adapter. Note that
        // this is skipped for `wasi-threads` since that's not supported in
        // components and it's also skipped for assemblyscript because that
        // doesn't support the wasip1 adapter.
        if !path.iter().any(|p| p == "wasm32-wasip3")
            && !path.iter().any(|p| p == "wasi-threads")
            && !path.iter().any(|p| p == "assemblyscript")
        {
            trials.push(Trial::test(
                format!("wasip1 adapter - {}", path.display()),
                move || run_test(&path, true).map_err(|e| format!("{e:?}").into()),
            ));
        }
    }
    Ok(())
}

fn run_test(path: &Path, componentize: bool) -> Result<()> {
    let wasmtime = Path::new(env!("CARGO_BIN_EXE_wasmtime"));
    let test_name = path.file_stem().unwrap().to_str().unwrap();
    let target_dir = wasmtime.parent().unwrap().parent().unwrap();
    let parent_dir = path.parent().ok_or(anyhow!("module has no parent?"))?;
    let spec = if let Ok(contents) = fs::read_to_string(&path.with_extension("json")) {
        serde_json::from_str(&contents)?
    } else {
        Spec::default()
    };

    let mut td = TempDir::new_in(&target_dir)?;
    td.disable_cleanup(true);
    let path = if componentize {
        let module = fs::read(path).expect("read wasm module");
        let component = ComponentEncoder::default()
            .module(module.as_slice())?
            .validate(true)
            .adapter(
                "wasi_snapshot_preview1",
                &fs::read(test_programs_artifacts::ADAPTER_COMMAND)?,
            )?
            .encode()?;
        let stem = path.file_stem().unwrap().to_str().unwrap();
        let component_path = td.path().join(format!("{stem}.component.wasm"));
        fs::write(&component_path, component)?;
        component_path
    } else {
        path.to_path_buf()
    };

    let Spec {
        args,
        dirs,
        env,
        exit_code: _,
        stderr: _,
        stdout: _,
    } = &spec;
    let mut cmd = wasmtime_test_util::command(wasmtime);
    cmd.arg("run");
    for dir in dirs {
        cmd.arg("--dir");
        let src = parent_dir.join(dir);
        let dst = td.path().join(dir);
        cp_r(&src, &dst)?;
        cmd.arg(format!("{}::{dir}", dst.display()));
    }
    for (k, v) in env {
        cmd.arg("--env");
        cmd.arg(format!("{k}={v}"));
    }
    let mut should_fail = KNOWN_FAILURES.contains(&test_name);
    if path.iter().any(|p| p == "wasm32-wasip3") {
        cmd.arg("-Sp3,http").arg("-Wcomponent-model-async");
        if !cfg!(feature = "component-model-async") {
            should_fail = true;
        }
    }
    cmd.arg(path);
    cmd.args(args);

    let result = cmd.output()?;
    td.disable_cleanup(true);
    let ok = spec == result;
    match (ok, should_fail) {
        // If this test passed and is not a known failure, or if it failed and
        // it's a known failure, then flag this test as "ok".
        (true, false) | (false, true) => Ok(()),

        // If this test failed and it's not known to fail, explain why.
        (false, false) => {
            td.disable_cleanup(false);
            let mut msg = String::new();
            writeln!(msg, "  command: {cmd:?}")?;
            writeln!(msg, "  spec: {spec:#?}")?;
            writeln!(msg, "  result.status: {}", result.status)?;
            if !result.stdout.is_empty() {
                write!(
                    msg,
                    "  result.stdout:\n    {}",
                    String::from_utf8_lossy(&result.stdout).replace("\n", "\n    ")
                )?;
            }
            if !result.stderr.is_empty() {
                writeln!(
                    msg,
                    "  result.stderr:\n    {}",
                    String::from_utf8_lossy(&result.stderr).replace("\n", "\n    ")
                )?;
            }
            anyhow::bail!("{msg}\nFAILED! The result does not match the specification");
        }

        // If this test passed but it's flagged as should be failed, then fail
        // this test for someone to update `KNOWN_FAILURES`.
        (true, true) => {
            anyhow::bail!("test passed but it's listed in `KNOWN_FAILURES`")
        }
    }
}

fn cp_r(path: &Path, dst: &Path) -> Result<()> {
    fs::create_dir(dst)?;
    for entry in path.read_dir()? {
        let entry = entry?;
        let path = entry.path();
        let dst = dst.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            cp_r(&path, &dst)?;
        } else {
            fs::copy(&path, &dst)?;
        }
    }
    Ok(())
}

#[derive(Debug, Default, Deserialize)]
struct Spec {
    #[serde(default)]
    args: Vec<String>,
    #[serde(default)]
    dirs: Vec<String>,
    #[serde(default)]
    env: HashMap<String, String>,
    exit_code: Option<i32>,
    stderr: Option<String>,
    stdout: Option<String>,
}

impl PartialEq<Output> for Spec {
    fn eq(&self, other: &Output) -> bool {
        self.exit_code.unwrap_or(0) == other.status.code().unwrap()
            && matches_or_missing(&self.stdout, &other.stdout)
            && matches_or_missing(&self.stderr, &other.stderr)
    }
}

fn matches_or_missing(a: &Option<String>, b: &[u8]) -> bool {
    a.as_ref()
        .map(|s| s == &String::from_utf8_lossy(b))
        .unwrap_or(true)
}
