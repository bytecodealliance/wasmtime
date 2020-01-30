//! Utility module to create trampolines in/out WebAssembly module.

mod create_handle;
mod func;
mod global;
mod memory;
mod table;
mod trap;

use self::func::create_handle_with_function;
use self::global::create_global;
use self::memory::create_handle_with_memory;
use self::table::create_handle_with_table;
use super::{Callable, FuncType, GlobalType, MemoryType, Store, TableType, Val};
use anyhow::Result;
use std::rc::Rc;

pub use self::global::GlobalState;

pub fn generate_func_export(
    ft: &FuncType,
    func: &Rc<dyn Callable + 'static>,
    store: &Store,
) -> Result<(wasmtime_runtime::InstanceHandle, wasmtime_runtime::Export)> {
    let instance = create_handle_with_function(ft, func, store)?;
    let export = instance.lookup("trampoline").expect("trampoline export");
    Ok((instance, export))
}

pub fn generate_global_export(
    gt: &GlobalType,
    val: Val,
) -> Result<(wasmtime_runtime::Export, GlobalState)> {
    create_global(gt, val)
}

pub fn generate_memory_export(
    m: &MemoryType,
) -> Result<(wasmtime_runtime::InstanceHandle, wasmtime_runtime::Export)> {
    let instance = create_handle_with_memory(m)?;
    let export = instance.lookup("memory").expect("memory export");
    Ok((instance, export))
}

pub fn generate_table_export(
    t: &TableType,
) -> Result<(wasmtime_runtime::InstanceHandle, wasmtime_runtime::Export)> {
    let instance = create_handle_with_table(t)?;
    let export = instance.lookup("table").expect("table export");
    Ok((instance, export))
}
