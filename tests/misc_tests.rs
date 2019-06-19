mod runtime;

use std::sync::{Once, ONCE_INIT};

static INIT: Once = ONCE_INIT;

fn setup() {
    INIT.call_once(|| {
        std::env::set_var("RUST_LOG", "wasi_common=trace");
        pretty_env_logger::init_custom_env("RUST_LOG");
    });
}

#[test]
fn sched_yield() {
    setup();
    runtime::run_wasm("tests/misc-testsuite/sched_yield.wasm")
}

#[test]
fn truncation_rights() {
    setup();
    runtime::run_wasm("tests/misc-testsuite/truncation_rights.wasm")
}

#[test]
fn unlink_dir() {
    setup();
    runtime::run_wasm("tests/misc-testsuite/unlink_dir.wasm")
}

#[test]
fn remove_nonempty_dir() {
    setup();
    runtime::run_wasm("tests/misc-testsuite/remove_nonempty_dir.wasm")
}

#[test]
fn interesting_paths() {
    setup();
    runtime::run_wasm("tests/misc-testsuite/interesting_paths.wasm")
}

#[test]
fn nofollow_errors() {
    setup();
    runtime::run_wasm("tests/misc-testsuite/nofollow_errors.wasm")
}

#[test]
fn symlink_loop() {
    setup();
    runtime::run_wasm("tests/misc-testsuite/symlink_loop.wasm")
}
