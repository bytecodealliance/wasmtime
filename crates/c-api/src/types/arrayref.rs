use crate::wasmtime_field_type_t;
use wasmtime::ArrayType;

pub struct wasmtime_array_type_t {
    pub(crate) ty: ArrayType,
}
wasmtime_c_api_macros::declare_own!(wasmtime_array_type_t);

#[unsafe(no_mangle)]
pub extern "C" fn wasmtime_array_type_new(
    engine: &crate::wasm_engine_t,
    field: &wasmtime_field_type_t,
) -> Box<wasmtime_array_type_t> {
    let ft = field.to_wasmtime();
    let ty = ArrayType::new(&engine.engine, ft);
    Box::new(wasmtime_array_type_t { ty })
}
