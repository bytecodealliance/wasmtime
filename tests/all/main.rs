#![expect(clippy::allow_attributes_without_reason, reason = "crate not migrated")]
#![cfg_attr(miri, allow(dead_code, unused_imports))]

use wasmtime::{
    ArrayRef, ArrayRefPre, AsContextMut, ExternRef, Result, Rooted, StructRef, StructRefPre, Val,
};

mod arrays;
mod async_functions;
mod call_hook;
mod cli_tests;
mod code_too_large;
mod component_model;
mod coredump;
mod custom_code_memory;
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
mod i31ref;
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
mod pulley;
mod relocs;
mod stack_creator;
mod stack_overflow;
mod store;
mod structs;
mod table;
mod tags;
mod threads;
mod traps;
mod types;
mod wait_notify;
mod wasi_testsuite;
mod winch_engine_features;

/// A helper to compile a module in a new store with reference types enabled.
pub(crate) fn ref_types_module(
    use_epochs: bool,
    source: &str,
) -> anyhow::Result<(wasmtime::Store<()>, wasmtime::Module)> {
    use wasmtime::*;

    let _ = env_logger::try_init();

    let mut config = Config::new();
    config.wasm_reference_types(true);

    config.wasm_tail_call(true);

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
    let mut config = wasmtime::PoolingAllocationConfig::new();

    config.total_memories(1);
    config.max_memory_size(1 << 16);
    config.total_tables(1);
    config.table_elements(10);

    // When testing, we may choose to start with MPK force-enabled to ensure
    // we use that functionality.
    if std::env::var("WASMTIME_TEST_FORCE_MPK").is_ok() {
        config.memory_protection_keys(wasmtime::MpkEnabled::Enable);
    }

    config.total_stacks(1);

    config
}

pub(crate) fn gc_store() -> Result<wasmtime::Store<()>> {
    let _ = env_logger::try_init();

    let mut config = wasmtime::Config::new();
    config.wasm_function_references(true);
    config.wasm_gc(true);

    let engine = wasmtime::Engine::new(&config)?;
    Ok(wasmtime::Store::new(&engine, ()))
}

pub(crate) fn retry_after_gc<S, T, U>(
    store: &mut S,
    value: T,
    mut f: impl FnMut(&mut S, T) -> Result<U>,
) -> Result<U>
where
    T: Send + Sync + 'static,
    S: AsContextMut,
{
    match f(store, value) {
        Ok(x) => Ok(x),
        Err(e) => match e.downcast::<wasmtime::GcHeapOutOfMemory<T>>() {
            Ok(oom) => {
                let (value, oom) = oom.take_inner();
                store.as_context_mut().gc(Some(&oom));
                f(store, value)
            }
            Err(e) => Err(e),
        },
    }
}

pub(crate) async fn retry_after_gc_async<S, T, U>(
    store: &mut S,
    value: T,
    mut f: impl FnMut(&mut S, T) -> Result<U>,
) -> Result<U>
where
    T: Send + Sync + 'static,
    S: AsContextMut,
    S::Data: Send + Sync + 'static,
{
    match f(store, value) {
        Ok(x) => Ok(x),
        Err(e) => match e.downcast::<wasmtime::GcHeapOutOfMemory<T>>() {
            Ok(oom) => {
                let (value, oom) = oom.take_inner();
                store.as_context_mut().gc_async(Some(&oom)).await;
                f(store, value)
            }
            Err(e) => Err(e),
        },
    }
}

pub(crate) fn new_externref<T>(store: &mut impl AsContextMut, value: T) -> Result<Rooted<ExternRef>>
where
    T: Send + Sync + 'static,
{
    retry_after_gc(store, value, |store, value| {
        ExternRef::new(store.as_context_mut(), value)
    })
}

pub(crate) async fn new_externref_async<S, T>(store: &mut S, value: T) -> Result<Rooted<ExternRef>>
where
    T: Send + Sync + 'static,
    S: AsContextMut,
    S::Data: Send + Sync + 'static,
{
    retry_after_gc_async(store, value, |store, value| {
        ExternRef::new(store.as_context_mut(), value)
    })
    .await
}

pub(crate) fn new_struct(
    store: &mut impl AsContextMut,
    pre: &StructRefPre,
    fields: &[Val],
) -> Result<Rooted<StructRef>> {
    retry_after_gc(store, (), |store, ()| StructRef::new(store, pre, fields))
}

pub(crate) fn new_array(
    store: &mut impl AsContextMut,
    pre: &ArrayRefPre,
    elem: &Val,
    len: u32,
) -> Result<Rooted<ArrayRef>> {
    retry_after_gc(store, (), |store, ()| ArrayRef::new(store, pre, elem, len))
}

pub(crate) fn new_fixed_array(
    store: &mut impl AsContextMut,
    pre: &ArrayRefPre,
    elems: &[Val],
) -> Result<Rooted<ArrayRef>> {
    retry_after_gc(store, (), |store, ()| {
        ArrayRef::new_fixed(store, pre, elems)
    })
}

trait ErrorExt {
    fn assert_contains(&self, msg: &str);
}

impl ErrorExt for anyhow::Error {
    fn assert_contains(&self, msg: &str) {
        if self.chain().any(|e| e.to_string().contains(msg)) {
            return;
        }

        panic!("failed to find:\n{msg}\n\nwithin error message:\n{self:?}")
    }
}
