//! Support for reading the value of a wasm global from outside the module.

use action::Value;
use cranelift_codegen::ir;
use cranelift_wasm::GlobalIndex;
use std::string::String;
use vmcontext::VMContext;
use wasmtime_environ::{Export, Module};

/// Jumps to the code region of memory and invoke the exported function
pub fn get(module: &Module, vmctx: *mut VMContext, global_name: &str) -> Result<Value, String> {
    let global_index = match module.exports.get(global_name) {
        Some(Export::Global(index)) => *index,
        Some(_) => return Err(format!("exported item \"{}\" is not a global", global_name)),
        None => return Err(format!("no export named \"{}\"", global_name)),
    };

    get_by_index(module, vmctx, global_index)
}

pub fn get_by_index(
    module: &Module,
    vmctx: *mut VMContext,
    global_index: GlobalIndex,
) -> Result<Value, String> {
    // TODO: Return Err if the index is out of bounds.
    unsafe {
        let vmctx = &mut *vmctx;
        let vmglobal = vmctx.global(global_index);
        let definition = vmglobal.get_definition(module.is_imported_global(global_index));
        Ok(match module.globals[global_index].ty {
            ir::types::I32 => Value::I32(*definition.as_i32()),
            ir::types::I64 => Value::I64(*definition.as_i64()),
            ir::types::F32 => Value::F32(*definition.as_f32_bits()),
            ir::types::F64 => Value::F64(*definition.as_f64_bits()),
            other => return Err(format!("global with type {} not supported", other)),
        })
    }
}
