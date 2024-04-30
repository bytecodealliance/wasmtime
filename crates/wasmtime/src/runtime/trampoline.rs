//! Utility module to create trampolines in/out WebAssembly module.

mod func;
mod global;
mod memory;
mod table;

pub use self::func::*;
pub use self::global::*;
pub(crate) use memory::MemoryCreatorProxy;

use self::memory::create_memory;
use self::table::create_table;
use crate::module::BareModuleInfo;
use crate::runtime::vm::{
    Imports, InstanceAllocationRequest, InstanceAllocator, OnDemandInstanceAllocator, SharedMemory,
    StorePtr, VMFunctionImport,
};
use crate::store::{InstanceId, StoreOpaque};
use crate::{MemoryType, TableType};
use anyhow::Result;
use std::any::Any;
use std::sync::Arc;
use wasmtime_environ::{MemoryIndex, Module, TableIndex, VMSharedTypeIndex};

fn create_handle(
    module: Module,
    store: &mut StoreOpaque,
    host_state: Box<dyn Any + Send + Sync>,
    func_imports: &[VMFunctionImport],
    one_signature: Option<VMSharedTypeIndex>,
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
        let allocator = OnDemandInstanceAllocator::new(config.mem_creator.clone(), 0);
        let handle = allocator.allocate_module(InstanceAllocationRequest {
            imports,
            host_state,
            store: StorePtr::new(store.traitobj()),
            runtime_info,
            wmemcheck: false,
            pkey: None,
        })?;

        Ok(store.add_dummy_instance(handle))
    }
}

pub fn generate_memory_export(
    store: &mut StoreOpaque,
    m: &MemoryType,
    preallocation: Option<&SharedMemory>,
) -> Result<crate::runtime::vm::ExportMemory> {
    let instance = create_memory(store, m, preallocation)?;
    Ok(store
        .instance_mut(instance)
        .get_exported_memory(MemoryIndex::from_u32(0)))
}

pub fn generate_table_export(
    store: &mut StoreOpaque,
    t: &TableType,
) -> Result<crate::runtime::vm::ExportTable> {
    let instance = create_table(store, t)?;
    Ok(store
        .instance_mut(instance)
        .get_exported_table(TableIndex::from_u32(0)))
}
