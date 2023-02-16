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
use crate::module::BareModuleInfo;
use crate::store::{InstanceId, StoreOpaque};
use crate::{GlobalType, MemoryType, TableType, Val};
use anyhow::Result;
use std::any::Any;
use std::sync::Arc;
use wasmtime_environ::{GlobalIndex, MemoryIndex, Module, TableIndex};
use wasmtime_runtime::{
    Imports, InstanceAllocationRequest, InstanceAllocator, OnDemandInstanceAllocator, SharedMemory,
    StorePtr, VMFunctionImport, VMSharedSignatureIndex,
};

fn create_handle(
    module: Module,
    store: &mut StoreOpaque,
    host_state: Box<dyn Any + Send + Sync>,
    func_imports: &[VMFunctionImport],
    one_signature: Option<VMSharedSignatureIndex>,
) -> Result<InstanceId> {
    let mut imports = Imports::default();
    imports.functions = func_imports;

    unsafe {
        let config = store.engine().config();
        // Use the on-demand allocator when creating handles associated with host objects
        // The configured instance allocator should only be used when creating module instances
        // as we don't want host objects to count towards instance limits.
        let module = Arc::new(module);
        let runtime_info =
            &BareModuleInfo::maybe_imported_func(module, one_signature).into_traitobj();
        let handle = OnDemandInstanceAllocator::new(config.mem_creator.clone(), 0).allocate(
            InstanceAllocationRequest {
                imports,
                host_state,
                store: StorePtr::new(store.traitobj()),
                runtime_info,
            },
        )?;

        Ok(store.add_instance(handle, true))
    }
}

pub fn generate_global_export(
    store: &mut StoreOpaque,
    gt: &GlobalType,
    val: Val,
) -> Result<wasmtime_runtime::ExportGlobal> {
    let instance = create_global(store, gt, val)?;
    Ok(store
        .instance_mut(instance)
        .get_exported_global(GlobalIndex::from_u32(0)))
}

pub fn generate_memory_export(
    store: &mut StoreOpaque,
    m: &MemoryType,
    preallocation: Option<&SharedMemory>,
) -> Result<wasmtime_runtime::ExportMemory> {
    let instance = create_memory(store, m, preallocation)?;
    Ok(store
        .instance_mut(instance)
        .get_exported_memory(MemoryIndex::from_u32(0)))
}

pub fn generate_table_export(
    store: &mut StoreOpaque,
    t: &TableType,
) -> Result<wasmtime_runtime::ExportTable> {
    let instance = create_table(store, t)?;
    Ok(store
        .instance_mut(instance)
        .get_exported_table(TableIndex::from_u32(0)))
}
