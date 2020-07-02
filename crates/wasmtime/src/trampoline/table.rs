use super::create_handle::create_handle;
use crate::trampoline::StoreInstanceHandle;
use crate::Store;
use crate::{TableType, ValType};
use anyhow::{bail, Result};
use wasmtime_environ::entity::PrimaryMap;
use wasmtime_environ::{wasm, EntityIndex, Module};

pub fn create_handle_with_table(store: &Store, table: &TableType) -> Result<StoreInstanceHandle> {
    let mut module = Module::new();

    let table = wasm::Table {
        wasm_ty: table.element().to_wasm_type(),
        minimum: table.limits().min(),
        maximum: table.limits().max(),
        ty: match table.element() {
            ValType::FuncRef => wasm::TableElementType::Func,
            _ => bail!("cannot support {:?} as a table element", table.element()),
        },
    };
    let tunable = Default::default();

    let table_plan = wasmtime_environ::TablePlan::for_table(table, &tunable);
    let table_id = module.local.table_plans.push(table_plan);
    module
        .exports
        .insert("table".to_string(), EntityIndex::Table(table_id));

    create_handle(
        module,
        store,
        PrimaryMap::new(),
        Default::default(),
        Box::new(()),
        PrimaryMap::new(),
    )
}
