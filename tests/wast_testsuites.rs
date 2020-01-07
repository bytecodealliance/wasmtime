use std::path::Path;
use wasmtime::{Config, Engine, HostRef, Store, Strategy};
use wasmtime_wast::WastContext;

include!(concat!(env!("OUT_DIR"), "/wast_testsuite_tests.rs"));

// Each of the tests included from `wast_testsuite_tests` will call this
// function which actually executes the `wast` test suite given the `strategy`
// to compile it.
fn run_wast(wast: &str, strategy: Strategy) -> anyhow::Result<()> {
    let wast = Path::new(wast);

    let mut cfg = Config::new();
    cfg.wasm_simd(wast.iter().any(|s| s == "simd"))
        .wasm_multi_value(wast.iter().any(|s| s == "multi-value"))
        .strategy(strategy)?
        .cranelift_debug_verifier(true);
    let store = HostRef::new(Store::new(&Engine::new(&cfg)));
    let mut wast_context = WastContext::new(store);
    wast_context.register_spectest()?;
    wast_context.run_file(wast)?;
    Ok(())
}
