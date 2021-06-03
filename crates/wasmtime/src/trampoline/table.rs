use crate::store::{InstanceId, StoreOpaque};
use crate::trampoline::create_handle;
use crate::{TableType, ValType};
use anyhow::{bail, Result};
use wasmtime_environ::entity::PrimaryMap;
use wasmtime_environ::{wasm, Module};

pub fn create_table(store: &mut StoreOpaque<'_>, table: &TableType) -> Result<InstanceId> {
    let mut module = Module::new();

    let table = wasm::Table {
        wasm_ty: table.element().to_wasm_type(),
        minimum: table.limits().min(),
        maximum: table.limits().max(),
        ty: match table.element() {
            ValType::FuncRef => wasm::TableElementType::Func,
            ValType::ExternRef => wasm::TableElementType::Val(wasmtime_runtime::ref_type()),
            _ => bail!("cannot support {:?} as a table element", table.element()),
        },
    };
    let tunable = Default::default();

    let table_plan = wasmtime_environ::TablePlan::for_table(table, &tunable);
    let table_id = module.table_plans.push(table_plan);
    // TODO: can this `exports.insert` get removed?
    module
        .exports
        .insert(String::new(), wasm::EntityIndex::Table(table_id));

    create_handle(module, store, PrimaryMap::new(), Box::new(()), &[], None)
}
