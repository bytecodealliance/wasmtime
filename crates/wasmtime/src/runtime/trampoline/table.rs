use crate::store::{InstanceId, StoreOpaque};
use crate::trampoline::create_handle;
use crate::TableType;
use anyhow::Result;
use wasmtime_environ::{EntityIndex, Module, TypeTrace};

pub fn create_table(store: &mut StoreOpaque, table: &TableType) -> Result<InstanceId> {
    let mut module = Module::new();

    let wasmtime_table = table.wasmtime_table().clone();
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

    create_handle(module, store, Box::new(()), &[], None)
}
