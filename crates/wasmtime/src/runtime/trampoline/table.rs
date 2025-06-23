use crate::prelude::*;
use crate::runtime::vm::{
    Imports, InstanceAllocationRequest, InstanceAllocator, ModuleRuntimeInfo,
    OnDemandInstanceAllocator, StorePtr,
};
use crate::store::{InstanceId, StoreOpaque};
use crate::TableType;
use alloc::sync::Arc;
use wasmtime_environ::{EntityIndex, Module, TypeTrace};

pub fn create_table(store: &mut StoreOpaque, table: &TableType) -> Result<InstanceId> {
    let mut module = Module::new();

    let wasmtime_table = *table.wasmtime_table();
    let tunables = store.engine().tunables();

    debug_assert!(
        wasmtime_table.wasm_ty.is_canonicalized_for_runtime_usage(),
        "should be canonicalized for runtime usage: {:?}",
        wasmtime_table.wasm_ty
    );

    let table_plan = wasmtime_environ::TablePlan::for_table(wasmtime_table, tunables);
    let table_id = module.table_plans.push(table_plan);

    // TODO: can this `exports.insert` get removed?
    module
        .exports
        .insert(String::new(), EntityIndex::Table(table_id));

    let imports = Imports::default();

    unsafe {
        let config = store.engine().config();
        // Use the on-demand allocator when creating handles associated with host objects
        // The configured instance allocator should only be used when creating module instances
        // as we don't want host objects to count towards instance limits.
        let module = Arc::new(module);
        let runtime_info = &ModuleRuntimeInfo::bare_with_registered_type(
            module,
            table.element().clone().into_registered_type(),
        );
        let allocator = OnDemandInstanceAllocator::new(config.mem_creator.clone(), 0);
        let handle = allocator.allocate_module(InstanceAllocationRequest {
            imports,
            host_state: Box::new(()),
            store: StorePtr::new(store.traitobj()),
            runtime_info,
            wmemcheck: false,
            pkey: None,
        })?;

        Ok(store.add_dummy_instance(handle))
    }
}
