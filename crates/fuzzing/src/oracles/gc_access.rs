//! Oracle for the `GcAccess` fuzz test case generator.

use crate::generators;
use wasmtime::*;

/// Run the given `GcAccess` test case.
///
/// Panics if the generated Wasm module's embedded assertions fail, which
/// indicates a GC bug.
pub fn gc_access(config: generators::Config, input: generators::GcAccess) {
    let wasm = input.to_wasm();
    crate::oracles::log_wasm(&wasm);

    let mut wasmtime_config = config.to_wasmtime();
    wasmtime_config.wasm_gc(true);
    wasmtime_config.wasm_function_references(true);
    wasmtime_config.wasm_reference_types(true);
    wasmtime_config.wasm_bulk_memory(true);

    let engine = match Engine::new(&wasmtime_config) {
        Ok(e) => e,
        Err(_) => return,
    };

    let module = match Module::new(&engine, &wasm) {
        Ok(m) => m,
        Err(_) => return,
    };

    let mut linker = Linker::new(&engine);
    linker
        .func_wrap(
            "wasmtime",
            "gc",
            |mut caller: Caller<'_, crate::oracles::StoreLimits>| {
                let _ = caller.gc(None);
            },
        )
        .unwrap();

    let limits = crate::oracles::StoreLimits::new();
    let mut store = Store::new(&engine, limits);
    store.limiter(|s| s as &mut dyn ResourceLimiter);
    config.configure_store_epoch_and_fuel(&mut store);

    let instance = match linker.instantiate(&mut store, &module) {
        Ok(i) => i,
        Err(_) => return,
    };

    let run = instance
        .get_typed_func::<(), ()>(&mut store, "run")
        .expect("should have `run` export");

    if let Err(e) = run.call(&mut store, ()) {
        let msg = format!("{e:?}");
        assert!(
            !msg.contains("wasm trap: wasm `unreachable` instruction executed"),
            "GC access assertion failure: {e:?}"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generators::GcAccess;

    #[test]
    fn gc_access_oracle_passes() {
        crate::test::test_n_times(1024, |config: generators::Config, u| {
            let input: GcAccess = u.arbitrary()?;
            gc_access(config, input);
            Ok(())
        })
    }
}
