//! Utility module to create trampolines in/out WebAssembly module.

mod func;
mod global;
mod memory;
mod table;

pub(crate) use memory::MemoryCreatorProxy;

pub use self::func::{create_function, create_raw_function};
use self::global::create_global;
use self::memory::create_memory;
use self::table::create_table;
use crate::{GlobalType, MemoryType, Store, TableType, Val};
use anyhow::Result;
use std::any::Any;
use std::ops::Deref;
use std::sync::Arc;
use wasmtime_environ::{entity::PrimaryMap, wasm, Module};
use wasmtime_runtime::{
    Imports, InstanceAllocationRequest, InstanceAllocator, InstanceHandle,
    OnDemandInstanceAllocator, VMExternRefActivationsTable, VMFunctionBody, VMFunctionImport,
    VMSharedSignatureIndex,
};

/// A wrapper around `wasmtime_runtime::InstanceHandle` which pairs it with the
/// `Store` that it's rooted within. The instance is deallocated when `Store` is
/// deallocated, so this is a safe handle in terms of memory management for the
/// `Store`.
pub struct StoreInstanceHandle {
    pub store: Store,
    pub handle: InstanceHandle,
}

impl Clone for StoreInstanceHandle {
    fn clone(&self) -> StoreInstanceHandle {
        StoreInstanceHandle {
            store: self.store.clone(),
            // Note should be safe because the lifetime of the instance handle
            // is tied to the `Store` which this is paired with.
            handle: unsafe { self.handle.clone() },
        }
    }
}

impl Deref for StoreInstanceHandle {
    type Target = InstanceHandle;
    fn deref(&self) -> &InstanceHandle {
        &self.handle
    }
}

fn create_handle(
    module: Module,
    store: &Store,
    finished_functions: PrimaryMap<wasm::DefinedFuncIndex, *mut [VMFunctionBody]>,
    host_state: Box<dyn Any>,
    func_imports: &[VMFunctionImport],
    shared_signature_id: Option<VMSharedSignatureIndex>,
) -> Result<StoreInstanceHandle> {
    let mut imports = Imports::default();
    imports.functions = func_imports;

    unsafe {
        let config = store.engine().config();
        // Use the on-demand allocator when creating handles associated with host objects
        // The configured instance allocator should only be used when creating module instances
        // as we don't want host objects to count towards instance limits.
        let handle = OnDemandInstanceAllocator::new(config.mem_creator.clone(), 0).allocate(
            InstanceAllocationRequest {
                module: Arc::new(module),
                finished_functions: &finished_functions,
                imports,
                shared_signatures: shared_signature_id.into(),
                host_state,
                interrupts: store.interrupts(),
                externref_activations_table: store.externref_activations_table()
                    as *const VMExternRefActivationsTable
                    as *mut _,
                stack_map_lookup: Some(std::mem::transmute(store.stack_map_lookup())),
            },
        )?;

        Ok(store.add_instance(handle, true))
    }
}

pub fn generate_global_export(
    store: &Store,
    gt: &GlobalType,
    val: Val,
) -> Result<(StoreInstanceHandle, wasmtime_runtime::ExportGlobal)> {
    let instance = create_global(store, gt, val)?;
    let idx = wasm::EntityIndex::Global(wasm::GlobalIndex::from_u32(0));
    match instance.lookup_by_declaration(&idx) {
        wasmtime_runtime::Export::Global(g) => Ok((instance, g)),
        _ => unreachable!(),
    }
}

pub fn generate_memory_export(
    store: &Store,
    m: &MemoryType,
) -> Result<(StoreInstanceHandle, wasmtime_runtime::ExportMemory)> {
    let instance = create_memory(store, m)?;
    let idx = wasm::EntityIndex::Memory(wasm::MemoryIndex::from_u32(0));
    match instance.lookup_by_declaration(&idx) {
        wasmtime_runtime::Export::Memory(m) => Ok((instance, m)),
        _ => unreachable!(),
    }
}

pub fn generate_table_export(
    store: &Store,
    t: &TableType,
) -> Result<(StoreInstanceHandle, wasmtime_runtime::ExportTable)> {
    let instance = create_table(store, t)?;
    let idx = wasm::EntityIndex::Table(wasm::TableIndex::from_u32(0));
    match instance.lookup_by_declaration(&idx) {
        wasmtime_runtime::Export::Table(t) => Ok((instance, t)),
        _ => unreachable!(),
    }
}
