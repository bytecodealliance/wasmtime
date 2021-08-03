#![no_main]

use libfuzzer_sys::fuzz_target;
use std::sync::atomic::{AtomicUsize, Ordering::SeqCst};
use wasmtime_fuzzing::{generators, oracles};

// Keep track of how many WebAssembly modules we actually executed (i.e. ran to
// completion) versus how many were tried.
static TRIED: AtomicUsize = AtomicUsize::new(0);
static EXECUTED: AtomicUsize = AtomicUsize::new(0);

fuzz_target!(|data: (
    generators::Config,
    wasm_smith::ConfiguredModule<oracles::SingleFunctionModuleConfig>
)| {
    let (config, mut wasm) = data;
    wasm.ensure_termination(1000);
    let tried = TRIED.fetch_add(1, SeqCst);
    let executed = match oracles::differential_spec_execution(&wasm.to_bytes(), &config) {
        Some(_) => EXECUTED.fetch_add(1, SeqCst),
        None => EXECUTED.load(SeqCst),
    };
    if tried > 0 && tried % 1000 == 0 {
        println!(
            "=== Execution rate ({} executed modules / {} tried modules): {}% ===",
            executed,
            tried,
            executed as f64 / tried as f64 * 100f64
        )
    }
});
