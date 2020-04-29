//! Utility module to create trampolines in/out WebAssembly module.

mod create_handle;
mod func;
mod global;
mod memory;
mod table;

pub(crate) use memory::MemoryCreatorProxy;

use self::func::create_handle_with_function;
use self::global::create_global;
use self::memory::create_handle_with_memory;
use self::table::create_handle_with_table;
use crate::{FuncType, GlobalType, MemoryType, Store, TableType, Trap, Val};
use anyhow::Result;
use std::any::Any;
use std::ops::Deref;
use wasmtime_runtime::{InstanceHandle, VMContext, VMFunctionBody, VMTrampoline};

/// A wrapper around `wasmtime_runtime::InstanceHandle` which pairs it with the
/// `Store` that it's rooted within. The instance is deallocated when `Store` is
/// deallocated, so this is a safe handle in terms of memory management for the
/// `Store`.
pub struct StoreInstanceHandle {
    pub store: Store,
    pub handle: InstanceHandle,
}

impl Clone for StoreInstanceHandle {
    fn clone(&self) -> StoreInstanceHandle {
        StoreInstanceHandle {
            store: self.store.clone(),
            // Note should be safe because the lifetime of the instance handle
            // is tied to the `Store` which this is paired with.
            handle: unsafe { self.handle.clone() },
        }
    }
}

impl Deref for StoreInstanceHandle {
    type Target = InstanceHandle;
    fn deref(&self) -> &InstanceHandle {
        &self.handle
    }
}

pub fn generate_func_export(
    ft: &FuncType,
    func: Box<dyn Fn(*mut VMContext, *mut u128) -> Result<(), Trap>>,
    store: &Store,
) -> Result<(
    StoreInstanceHandle,
    wasmtime_runtime::ExportFunction,
    VMTrampoline,
)> {
    let (instance, trampoline) = create_handle_with_function(ft, func, store)?;
    match instance.lookup("trampoline").expect("trampoline export") {
        wasmtime_runtime::Export::Function(f) => Ok((instance, f, trampoline)),
        _ => unreachable!(),
    }
}

/// Note that this is `unsafe` since `func` must be a valid function pointer and
/// have a signature which matches `ft`, otherwise the returned
/// instance/export/etc may exhibit undefined behavior.
pub unsafe fn generate_raw_func_export(
    ft: &FuncType,
    func: *mut [VMFunctionBody],
    trampoline: VMTrampoline,
    store: &Store,
    state: Box<dyn Any>,
) -> Result<(StoreInstanceHandle, wasmtime_runtime::ExportFunction)> {
    let instance = func::create_handle_with_raw_function(ft, func, trampoline, store, state)?;
    match instance.lookup("trampoline").expect("trampoline export") {
        wasmtime_runtime::Export::Function(f) => Ok((instance, f)),
        _ => unreachable!(),
    }
}

pub fn generate_global_export(
    store: &Store,
    gt: &GlobalType,
    val: Val,
) -> Result<(StoreInstanceHandle, wasmtime_runtime::ExportGlobal)> {
    let instance = create_global(store, gt, val)?;
    match instance.lookup("global").expect("global export") {
        wasmtime_runtime::Export::Global(g) => Ok((instance, g)),
        _ => unreachable!(),
    }
}

pub fn generate_memory_export(
    store: &Store,
    m: &MemoryType,
) -> Result<(StoreInstanceHandle, wasmtime_runtime::ExportMemory)> {
    let instance = create_handle_with_memory(store, m)?;
    match instance.lookup("memory").expect("memory export") {
        wasmtime_runtime::Export::Memory(m) => Ok((instance, m)),
        _ => unreachable!(),
    }
}

pub fn generate_table_export(
    store: &Store,
    t: &TableType,
) -> Result<(StoreInstanceHandle, wasmtime_runtime::ExportTable)> {
    let instance = create_handle_with_table(store, t)?;
    match instance.lookup("table").expect("table export") {
        wasmtime_runtime::Export::Table(t) => Ok((instance, t)),
        _ => unreachable!(),
    }
}
