#![cfg_attr(miri, allow(dead_code, unused_imports))]

mod async_functions;
mod call_hook;
mod cli_tests;
mod code_too_large;
mod component_model;
mod coredump;
mod debug;
mod defaults;
mod epoch_interruption;
mod externals;
mod fuel;
mod func;
mod funcref;
mod gc;
mod globals;
mod host_funcs;
mod iloop;
mod import_calling_export;
mod import_indexes;
mod instance;
mod invoke_func_via_table;
mod limits;
mod linker;
mod memory;
mod memory_creator;
mod module;
mod module_serialize;
mod name;
mod noextern;
mod piped_tests;
mod pooling_allocator;
mod relocs;
mod stack_creator;
mod stack_overflow;
mod store;
mod table;
mod threads;
mod traps;
mod types;
mod wait_notify;
mod wasi_testsuite;
mod wast;
// Currently Winch is only supported in x86_64.
#[cfg(all(target_arch = "x86_64"))]
mod winch;

/// A helper to compile a module in a new store with reference types enabled.
pub(crate) fn ref_types_module(
    use_epochs: bool,
    source: &str,
) -> anyhow::Result<(wasmtime::Store<()>, wasmtime::Module)> {
    use wasmtime::*;

    let _ = env_logger::try_init();

    let mut config = Config::new();
    config.wasm_reference_types(true);

    if !cfg!(target_arch = "s390x") {
        // TODO(6530): s390x doesn't support tail calls yet.
        config.wasm_tail_call(true);
    }

    if use_epochs {
        config.epoch_interruption(true);
    }

    let engine = Engine::new(&config)?;
    let mut store = Store::new(&engine, ());
    if use_epochs {
        store.set_epoch_deadline(1);
    }

    let module = Module::new(&engine, source)?;

    Ok((store, module))
}

/// A helper determining whether the pooling allocator tests should be skipped.
pub(crate) fn skip_pooling_allocator_tests() -> bool {
    // There are a couple of issues when running the pooling allocator tests under QEMU:
    // - high memory usage that may exceed the limits imposed by the environment (e.g. CI)
    // - https://github.com/bytecodealliance/wasmtime/pull/2518#issuecomment-747280133
    std::env::var("WASMTIME_TEST_NO_HOG_MEMORY").is_ok()
}

/// Get the default pooling allocator configuration for tests, which is a
/// smaller pool than the normal default.
pub(crate) fn small_pool_config() -> wasmtime::PoolingAllocationConfig {
    let mut config = wasmtime::PoolingAllocationConfig::default();

    config.total_memories(1);
    config.memory_pages(1);
    config.total_tables(1);
    config.table_elements(10);

    // When testing, we may choose to start with MPK force-enabled to ensure
    // we use that functionality.
    if std::env::var("WASMTIME_TEST_FORCE_MPK").is_ok() {
        config.memory_protection_keys(wasmtime::MpkEnabled::Enable);
    }

    #[cfg(feature = "async")]
    config.total_stacks(1);

    config
}
