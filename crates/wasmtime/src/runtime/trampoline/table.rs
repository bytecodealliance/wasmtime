use crate::TableType;
use crate::prelude::*;
use crate::runtime::vm::{Imports, ModuleRuntimeInfo, OnDemandInstanceAllocator};
use crate::store::{AllocateInstanceKind, InstanceId, StoreOpaque, StoreResourceLimiter};
use alloc::sync::Arc;
use wasmtime_environ::StaticModuleIndex;
use wasmtime_environ::{EntityIndex, Module, TypeTrace};

pub async fn create_table(
    store: &mut StoreOpaque,
    limiter: Option<&mut StoreResourceLimiter<'_>>,
    table: &TableType,
) -> Result<InstanceId> {
    let mut module = Module::new(StaticModuleIndex::from_u32(0));

    let wasmtime_table = *table.wasmtime_table();

    debug_assert!(
        wasmtime_table.ref_type.is_canonicalized_for_runtime_usage(),
        "should be canonicalized for runtime usage: {:?}",
        wasmtime_table.ref_type
    );

    let table_id = module.tables.push(wasmtime_table)?;

    // TODO: can this `exports.insert` get removed?
    let name = module.strings.insert("")?;
    module.exports.insert(name, EntityIndex::Table(table_id))?;

    let imports = Imports::default();

    unsafe {
        let allocator =
            OnDemandInstanceAllocator::new(store.engine().config().mem_creator.clone(), 0, false);
        let module = Arc::new(module);
        store
            .allocate_instance(
                limiter,
                AllocateInstanceKind::Dummy {
                    allocator: &allocator,
                },
                &ModuleRuntimeInfo::bare_with_registered_type(
                    module,
                    table.element().clone().into_registered_type(),
                )?,
                imports,
            )
            .await
    }
}
