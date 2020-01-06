use std::path::Path;
use wasmtime::{Config, Engine, HostRef, Store};
use wasmtime_environ::settings;
use wasmtime_environ::settings::Configurable;
use wasmtime_jit::CompilationStrategy;
use wasmtime_wast::WastContext;

include!(concat!(env!("OUT_DIR"), "/wast_testsuite_tests.rs"));

// Each of the tests included from `wast_testsuite_tests` will call this
// function which actually executes the `wast` test suite given the `strategy`
// to compile it.
fn run_wast(wast: &str, strategy: CompilationStrategy) -> anyhow::Result<()> {
    let wast = Path::new(wast);

    let mut flag_builder = settings::builder();
    flag_builder.enable("enable_verifier").unwrap();
    flag_builder.enable("avoid_div_traps").unwrap();
    flag_builder.enable("enable_simd").unwrap();

    let mut cfg = Config::new();
    cfg.wasm_simd(wast.iter().any(|s| s == "simd"))
        .wasm_multi_value(wast.iter().any(|s| s == "multi-value"))
        .strategy(strategy)
        .flags(settings::Flags::new(flag_builder));
    let store = HostRef::new(Store::new(&Engine::new(&cfg)));
    let mut wast_context = WastContext::new(store);
    wast_context.register_spectest()?;
    wast_context.run_file(wast)?;
    Ok(())
}
