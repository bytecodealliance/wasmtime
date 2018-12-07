//! Support for reading the value of a wasm global from outside the module.

use action::{ActionError, RuntimeValue};
use cranelift_codegen::ir;
use cranelift_entity::EntityRef;
use cranelift_wasm::GlobalIndex;
use instance::Instance;
use wasmtime_environ::{Export, Module};

/// Reads the value of the named global variable in `module`.
pub fn get(
    module: &Module,
    instance: &mut Instance,
    global_name: &str,
) -> Result<RuntimeValue, ActionError> {
    let global_index = match module.exports.get(global_name) {
        Some(Export::Global(index)) => *index,
        Some(_) => {
            return Err(ActionError::Kind(format!(
                "exported item \"{}\" is not a global",
                global_name
            )))
        }
        None => {
            return Err(ActionError::Field(format!(
                "no export named \"{}\"",
                global_name
            )))
        }
    };

    get_by_index(module, instance, global_index)
}

/// Reads the value of the indexed global variable in `module`.
pub fn get_by_index(
    module: &Module,
    instance: &mut Instance,
    global_index: GlobalIndex,
) -> Result<RuntimeValue, ActionError> {
    unsafe {
        let vmctx = &mut *instance.vmctx();
        let vmglobal = vmctx.global(global_index);
        let definition = vmglobal.get_definition(module.is_imported_global(global_index));
        Ok(
            match module
                .globals
                .get(global_index)
                .ok_or_else(|| ActionError::Index(global_index.index() as u64))?
                .ty
            {
                ir::types::I32 => RuntimeValue::I32(*definition.as_i32()),
                ir::types::I64 => RuntimeValue::I64(*definition.as_i64()),
                ir::types::F32 => RuntimeValue::F32(*definition.as_f32_bits()),
                ir::types::F64 => RuntimeValue::F64(*definition.as_f64_bits()),
                other => {
                    return Err(ActionError::Type(format!(
                        "global with type {} not supported",
                        other
                    )))
                }
            },
        )
    }
}
