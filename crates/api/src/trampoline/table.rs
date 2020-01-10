use super::create_handle::create_handle;
use crate::{TableType, ValType};
use anyhow::{bail, Result};
use wasmtime_environ::entity::PrimaryMap;
use wasmtime_environ::{wasm, Module};
use wasmtime_runtime::InstanceHandle;

pub fn create_handle_with_table(table: &TableType) -> Result<InstanceHandle> {
    let mut module = Module::new();

    let table = wasm::Table {
        minimum: table.limits().min(),
        maximum: table.limits().max(),
        ty: match table.element() {
            ValType::FuncRef => wasm::TableElementType::Func,
            _ => match table.element().get_wasmtime_type() {
                Some(t) => wasm::TableElementType::Val(t),
                None => bail!("cannot support {:?} as a table element", table.element()),
            },
        },
    };
    let tunable = Default::default();

    let table_plan = wasmtime_environ::TablePlan::for_table(table, &tunable);
    let table_id = module.table_plans.push(table_plan);
    module.exports.insert(
        "table".to_string(),
        wasmtime_environ::Export::Table(table_id),
    );

    create_handle(module, None, PrimaryMap::new(), Box::new(()))
}
