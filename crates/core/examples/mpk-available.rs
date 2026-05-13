//! Exit 0 if Wasmtime's MPK runtime support is available on this host,
//! 1 otherwise. This shares the detection logic with `wasmtime`'s runtime
//! `is_supported()` check via [`wasmtime_internal_core::mpk::is_supported`],
//! so CI's "should we set `WASMTIME_TEST_FORCE_MPK=1`?" decision can never
//! drift from what the runtime itself reports.

use std::process::exit;

fn main() {
    if wasmtime_internal_core::mpk::is_supported() {
        eprintln!("MPK is available");
        exit(0);
    } else {
        eprintln!("MPK is not available");
        exit(1);
    }
}
