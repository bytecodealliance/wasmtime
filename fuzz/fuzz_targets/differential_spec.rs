#![no_main]

use libfuzzer_sys::arbitrary::{Result, Unstructured};
use libfuzzer_sys::fuzz_target;
use std::sync::atomic::{AtomicUsize, Ordering::SeqCst};
use wasmtime_fuzzing::{generators, oracles};

// Keep track of how many WebAssembly modules we actually executed (i.e. ran to
// completion) versus how many were tried.
static TRIED: AtomicUsize = AtomicUsize::new(0);
static EXECUTED: AtomicUsize = AtomicUsize::new(0);

fuzz_target!(|data: &[u8]| {
    // errors in `run` have to do with not enough input in `data`, which we
    // ignore here since it doesn't affect how we'd like to fuzz.
    drop(run(data));
});

fn run(data: &[u8]) -> Result<()> {
    let mut u = Unstructured::new(data);
    let mut config: generators::Config = u.arbitrary()?;
    config.set_differential_config();

    // Enable features that the spec interpreter has implemented
    config.module_config.config.simd_enabled = true;

    // TODO: this is a best-effort attempt to avoid errors caused by the
    //       generated module exporting no functions.
    config.module_config.config.min_exports = 5;
    config.module_config.config.max_exports = 5;

    let module = config.generate(&mut u, Some(1000))?;
    let tried = TRIED.fetch_add(1, SeqCst);
    let executed = match oracles::differential_spec_execution(&module.to_bytes(), &config) {
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
    Ok(())
}
