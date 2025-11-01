use crate::{CExternType, wasm_engine_t, wasm_exporttype_t, wasm_importtype_t};
use wasmtime::component::types::Module;

type_wrapper! {
    pub struct wasmtime_module_type_t {
        pub(crate) ty: Module,
    }

    clone: wasmtime_module_type_clone,
    delete: wasmtime_module_type_delete,
}

#[unsafe(no_mangle)]
pub extern "C" fn wasmtime_module_type_import_count(
    ty: &wasmtime_module_type_t,
    engine: &wasm_engine_t,
) -> usize {
    ty.ty.imports(&engine.engine).len()
}

#[unsafe(no_mangle)]
pub extern "C" fn wasmtime_module_type_import_nth<'a>(
    ty: &'a wasmtime_module_type_t,
    engine: &wasm_engine_t,
    nth: usize,
) -> Option<Box<wasm_importtype_t>> {
    let ((module, field), item) = ty.ty.imports(&engine.engine).nth(nth)?;
    Some(Box::new(wasm_importtype_t::new(
        module.to_string(),
        field.to_string(),
        CExternType::new(item),
    )))
}

#[unsafe(no_mangle)]
pub extern "C" fn wasmtime_module_type_export_count(
    ty: &wasmtime_module_type_t,
    engine: &wasm_engine_t,
) -> usize {
    ty.ty.exports(&engine.engine).len()
}

#[unsafe(no_mangle)]
pub extern "C" fn wasmtime_module_type_export_nth<'a>(
    ty: &'a wasmtime_module_type_t,
    engine: &wasm_engine_t,
    nth: usize,
) -> Option<Box<wasm_exporttype_t>> {
    let (name, item) = ty.ty.exports(&engine.engine).nth(nth)?;
    Some(Box::new(wasm_exporttype_t::new(
        name.to_string(),
        CExternType::new(item),
    )))
}
