//! Utility module to create trampolines in/out WebAssembly module.

mod code_memory;
mod create_handle;
mod func;
mod global;
mod memory;

use failure::Error;
use std::cell::RefCell;
use std::rc::Rc;

use self::func::create_handle_with_function;
use self::global::create_global;
use self::memory::create_handle_with_memory;
use super::{Func, GlobalType, MemoryType, Val};

pub use self::global::GlobalState;

pub fn generate_func_export(f: &Rc<RefCell<Func>>) -> Result<(), Error> {
    let mut instance = create_handle_with_function(f)?;
    let export = instance.lookup("trampoline").expect("trampoline export");

    f.borrow_mut().anchor = Some((instance, export));
    Ok(())
}

pub fn generate_global_export(
    gt: &GlobalType,
    val: Val,
) -> Result<(wasmtime_runtime::Export, GlobalState), Error> {
    create_global(gt, val)
}

pub fn generate_memory_export(
    m: &MemoryType,
) -> Result<(wasmtime_runtime::InstanceHandle, wasmtime_runtime::Export), Error> {
    let mut instance = create_handle_with_memory(m)?;
    let export = instance.lookup("memory").expect("memory export");
    Ok((instance, export))
}
