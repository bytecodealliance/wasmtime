mod runtime;
mod utils;

use std::path::Path;

fn run_test_with_workspace<P: AsRef<Path>>(path: P) -> Result<(), String> {
    // Load in the wasm testcase
    let data = utils::read_wasm(path.as_ref())?;
    let bin_name = utils::extract_exec_name_from_path(path.as_ref())?;

    // Prepare workspace
    let workspace = utils::prepare_workspace(&bin_name)?;

    // Run!
    runtime::instantiate(&data, bin_name, Some(workspace))
}

fn run_test_without_workspace<P: AsRef<Path>>(path: P) -> Result<(), String> {
    // Load in the wasm testcase
    let data = utils::read_wasm(path.as_ref())?;
    let bin_name = utils::extract_exec_name_from_path(path.as_ref())?;

    // Run!
    runtime::instantiate(&data, bin_name, None)
}

#[cfg(all(unix))]
#[test]
fn sched_yield() -> Result<(), String> {
    run_test_without_workspace("tests/misc-tests/bin/sched_yield.wasm")
}

#[cfg(all(unix))]
#[test]
fn truncation_rights() -> Result<(), String> {
    run_test_with_workspace("tests/misc-tests/bin/truncation_rights.wasm")
}

#[cfg(all(unix))]
#[test]
fn unlink_directory() -> Result<(), String> {
    run_test_with_workspace("tests/misc-tests/bin/unlink_directory.wasm")
}

#[cfg(all(unix))]
#[test]
fn remove_nonempty_directory() -> Result<(), String> {
    run_test_with_workspace("tests/misc-tests/bin/remove_nonempty_directory.wasm")
}

#[cfg(all(unix))]
#[test]
fn interesting_paths() -> Result<(), String> {
    run_test_with_workspace("tests/misc-tests/bin/interesting_paths.wasm")
}

#[cfg(all(unix))]
#[test]
fn nofollow_errors() -> Result<(), String> {
    run_test_with_workspace("tests/misc-tests/bin/nofollow_errors.wasm")
}

#[cfg(all(unix))]
#[test]
fn symlink_loop() -> Result<(), String> {
    run_test_with_workspace("tests/misc-tests/bin/symlink_loop.wasm")
}

#[cfg(all(unix))]
#[test]
fn close_preopen() -> Result<(), String> {
    run_test_with_workspace("tests/misc-tests/bin/close_preopen.wasm")
}

#[cfg(all(unix))]
#[test]
fn clock_time_get() -> Result<(), String> {
    run_test_without_workspace("tests/misc-tests/bin/clock_time_get.wasm")
}

#[cfg(all(unix))]
#[test]
fn readlink_no_buffer() -> Result<(), String> {
    run_test_with_workspace("tests/misc-tests/bin/readlink_no_buffer.wasm")
}

#[cfg(all(unix))]
#[test]
fn isatty() -> Result<(), String> {
    run_test_with_workspace("tests/misc-tests/bin/isatty.wasm")
}

#[cfg(all(unix))]
#[test]
fn directory_seek() -> Result<(), String> {
    run_test_with_workspace("tests/misc-tests/bin/directory_seek.wasm")
}

#[test]
fn big_random_buf() -> Result<(), String> {
    run_test_without_workspace("tests/misc-tests/bin/big_random_buf.wasm")
}
