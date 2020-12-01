use std::path::Path;
use wasmtime::{Config, Engine, Store, Strategy};
use wasmtime_wast::WastContext;

include!(concat!(env!("OUT_DIR"), "/wast_testsuite_tests.rs"));

// Each of the tests included from `wast_testsuite_tests` will call this
// function which actually executes the `wast` test suite given the `strategy`
// to compile it.
fn run_wast(wast: &str, strategy: Strategy) -> anyhow::Result<()> {
    let wast = Path::new(wast);

    let simd = wast.iter().any(|s| s == "simd");

    let multi_memory = wast.iter().any(|s| s == "multi-memory");
    let module_linking = wast.iter().any(|s| s == "module-linking");
    let bulk_mem = multi_memory || wast.iter().any(|s| s == "bulk-memory-operations");

    // Some simd tests assume support for multiple tables, which are introduced
    // by reference types.
    let reftypes = simd || wast.iter().any(|s| s == "reference-types");

    let mut cfg = Config::new();
    cfg.wasm_simd(simd)
        .wasm_bulk_memory(bulk_mem)
        .wasm_reference_types(reftypes)
        .wasm_multi_memory(multi_memory)
        .wasm_module_linking(module_linking)
        .strategy(strategy)?
        .cranelift_debug_verifier(true);

    // By default we'll allocate huge chunks (6gb) of the address space for each
    // linear memory. This is typically fine but when we emulate tests with QEMU
    // it turns out that it causes memory usage to balloon massively. Leave a
    // knob here so on CI we can cut down the memory usage of QEMU and avoid the
    // OOM killer.
    //
    // Locally testing this out this drops QEMU's memory usage running this
    // tests suite from 10GiB to 600MiB. Previously we saw that crossing the
    // 10GiB threshold caused our processes to get OOM killed on CI.
    if std::env::var("WASMTIME_TEST_NO_HOG_MEMORY").is_ok() {
        cfg.static_memory_maximum_size(0);
    }

    let store = Store::new(&Engine::new(&cfg));
    let mut wast_context = WastContext::new(store);
    wast_context.register_spectest()?;
    wast_context.run_file(wast)?;
    Ok(())
}
