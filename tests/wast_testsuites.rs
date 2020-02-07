use std::path::Path;
use wasmtime::{Config, Engine, OptLevel, Store, Strategy};
use wasmtime_wast::WastContext;

include!(concat!(env!("OUT_DIR"), "/wast_testsuite_tests.rs"));

// Each of the tests included from `wast_testsuite_tests` will call this
// function which actually executes the `wast` test suite given the `strategy`
// to compile it.
fn run_wast(wast: &str, strategy: Strategy) -> anyhow::Result<()> {
    let wast = Path::new(wast);

    let simd = wast.iter().any(|s| s == "simd");

    // Some simd tests assume support for multiple tables, which are introduced
    // by reference types.
    let reftypes = simd || wast.iter().any(|s| s == "reference-types");

    // Reference types assumes support for bulk memory.
    let bulk_mem = reftypes
        || wast.iter().any(|s| s == "bulk-memory-operations")
        || wast.iter().any(|s| s == "table_copy.wast")
        || wast
            .iter()
            .any(|s| s == "table_copy_on_imported_tables.wast");

    // And bulk memory also assumes support for reference types (e.g. multiple
    // tables).
    let reftypes = reftypes || bulk_mem;

    let multi_val = wast.iter().any(|s| s == "multi-value");

    let mut cfg = Config::new();
    cfg.wasm_simd(simd)
        .wasm_reference_types(reftypes)
        .wasm_multi_value(multi_val)
        .wasm_bulk_memory(bulk_mem)
        .strategy(strategy)?
        .cranelift_debug_verifier(true);

    // FIXME: https://github.com/bytecodealliance/cranelift/issues/1409
    if simd {
        cfg.cranelift_opt_level(OptLevel::None);
    }

    let store = Store::new(&Engine::new(&cfg));
    let mut wast_context = WastContext::new(store);
    wast_context.register_spectest()?;
    wast_context.run_file(wast)?;
    Ok(())
}
