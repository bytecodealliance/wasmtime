use crate::store::{InstanceId, StoreOpaque};
use crate::trampoline::create_handle;
use crate::TableType;
use anyhow::Result;
use wasmtime_environ::entity::PrimaryMap;
use wasmtime_environ::{wasm, Module};

pub fn create_table(store: &mut StoreOpaque<'_>, table: &TableType) -> Result<InstanceId> {
    let mut module = Module::new();
    let table_plan = wasmtime_environ::TablePlan::for_table(
        table.wasmtime_table().clone(),
        &store.engine().config().tunables,
    );
    let table_id = module.table_plans.push(table_plan);
    // TODO: can this `exports.insert` get removed?
    module
        .exports
        .insert(String::new(), wasm::EntityIndex::Table(table_id));

    create_handle(module, store, PrimaryMap::new(), Box::new(()), &[], None)
}
