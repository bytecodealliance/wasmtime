//! Utility module to create trampolines in/out WebAssembly module.

mod func;
mod global;
mod memory;
mod table;

pub(crate) use memory::MemoryCreatorProxy;

pub use self::func::*;
use self::global::create_global;
use self::memory::create_memory;
use self::table::create_table;
use crate::store::{InstanceId, StoreOpaque};
use crate::{GlobalType, MemoryType, TableType, Val};
use anyhow::Result;
use std::any::Any;
use std::sync::Arc;
use wasmtime_environ::{EntityIndex, GlobalIndex, MemoryIndex, Module, TableIndex};
use wasmtime_runtime::{
    Imports, InstanceAllocationRequest, InstanceAllocator, OnDemandInstanceAllocator,
    VMFunctionImport, VMSharedSignatureIndex,
};

fn create_handle(
    module: Module,
    store: &mut StoreOpaque<'_>,
    host_state: Box<dyn Any + Send + Sync>,
    func_imports: &[VMFunctionImport],
    shared_signature_id: Option<VMSharedSignatureIndex>,
) -> Result<InstanceId> {
    let mut imports = Imports::default();
    imports.functions = func_imports;
    let functions = &Default::default();

    unsafe {
        let config = store.engine().config();
        // Use the on-demand allocator when creating handles associated with host objects
        // The configured instance allocator should only be used when creating module instances
        // as we don't want host objects to count towards instance limits.
        let handle = OnDemandInstanceAllocator::new(config.mem_creator.clone(), 0).allocate(
            InstanceAllocationRequest {
                module: Arc::new(module),
                functions,
                image_base: 0,
                imports,
                shared_signatures: shared_signature_id.into(),
                host_state,
                store: Some(store.traitobj),
                wasm_data: &[],
            },
        )?;

        Ok(store.add_instance(handle, true))
    }
}

pub fn generate_global_export(
    store: &mut StoreOpaque<'_>,
    gt: &GlobalType,
    val: Val,
) -> Result<wasmtime_runtime::ExportGlobal> {
    let instance = create_global(store, gt, val)?;
    let idx = EntityIndex::Global(GlobalIndex::from_u32(0));
    match store.instance(instance).lookup_by_declaration(&idx) {
        wasmtime_runtime::Export::Global(g) => Ok(g),
        _ => unreachable!(),
    }
}

pub fn generate_memory_export(
    store: &mut StoreOpaque<'_>,
    m: &MemoryType,
) -> Result<wasmtime_runtime::ExportMemory> {
    let instance = create_memory(store, m)?;
    let idx = EntityIndex::Memory(MemoryIndex::from_u32(0));
    match store.instance(instance).lookup_by_declaration(&idx) {
        wasmtime_runtime::Export::Memory(m) => Ok(m),
        _ => unreachable!(),
    }
}

pub fn generate_table_export(
    store: &mut StoreOpaque<'_>,
    t: &TableType,
) -> Result<wasmtime_runtime::ExportTable> {
    let instance = create_table(store, t)?;
    let idx = EntityIndex::Table(TableIndex::from_u32(0));
    match store.instance(instance).lookup_by_declaration(&idx) {
        wasmtime_runtime::Export::Table(t) => Ok(t),
        _ => unreachable!(),
    }
}
