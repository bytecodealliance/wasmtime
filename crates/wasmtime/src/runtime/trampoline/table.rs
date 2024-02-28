use crate::store::{InstanceId, StoreOpaque};
use crate::trampoline::create_handle;
use crate::TableType;
use anyhow::Result;
use wasmtime_environ::{EngineOrModuleTypeIndex, EntityIndex, Module, TypeTrace};

pub fn create_table(store: &mut StoreOpaque, table: &TableType) -> Result<InstanceId> {
    let mut module = Module::new();

    let wasmtime_table = table.wasmtime_table().clone();
    let tunables = store.engine().tunables();

    if cfg!(debug_assertions) {
        wasmtime_table
            .wasm_ty
            .trace(&mut |idx| match idx {
                EngineOrModuleTypeIndex::Engine(_) => Ok(()),
                EngineOrModuleTypeIndex::Module(module_idx) => Err(format!(
                    "found module-level canonicalized type index: {module_idx:?}"
                )),
            })
            .expect("element type should be engine-level canonicalized");
    }

    let table_plan = wasmtime_environ::TablePlan::for_table(wasmtime_table, tunables);
    let table_id = module.table_plans.push(table_plan);

    // TODO: can this `exports.insert` get removed?
    module
        .exports
        .insert(String::new(), EntityIndex::Table(table_id));

    create_handle(module, store, Box::new(()), &[], None)
}
