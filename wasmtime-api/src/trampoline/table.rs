use alloc::boxed::Box;
use alloc::string::ToString;
use cranelift_entity::PrimaryMap;
use cranelift_wasm::TableElementType;
use failure::Error;
use wasmtime_environ::Module;
use wasmtime_runtime::InstanceHandle;

use super::create_handle::create_handle;
use crate::{TableType, ValType};

pub fn create_handle_with_table(table: &TableType) -> Result<InstanceHandle, Error> {
    let mut module = Module::new();

    let table = cranelift_wasm::Table {
        minimum: table.limits().min(),
        maximum: if table.limits().max() == core::u32::MAX {
            None
        } else {
            Some(table.limits().max())
        },
        ty: match table.element() {
            ValType::FuncRef => TableElementType::Func,
            _ => TableElementType::Val(table.element().get_cranelift_type()),
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
