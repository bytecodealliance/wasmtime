use crate::{wasm_tagtype_t, wasm_valtype_vec_t, wasmtime_error_t};
use std::mem::MaybeUninit;
use wasmtime::ExnType;

#[derive(Clone)]
pub struct wasmtime_exn_type_t {
    pub(crate) ty: ExnType,
}
wasmtime_c_api_macros::declare_ty!(wasmtime_exn_type_t);

#[unsafe(no_mangle)]
pub extern "C" fn wasmtime_exn_type_new(
    engine: &crate::wasm_engine_t,
    params: &wasm_valtype_vec_t,
    out: &mut MaybeUninit<Box<wasmtime_exn_type_t>>,
) -> Option<Box<wasmtime_error_t>> {
    crate::handle_result(
        ExnType::new(
            &engine.engine,
            params
                .as_slice()
                .iter()
                .map(|t| (**t.as_ref().unwrap()).clone()),
        ),
        |ty| {
            out.write(Box::new(wasmtime_exn_type_t { ty }));
        },
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn wasmtime_exn_type_tag_type(ty: &wasmtime_exn_type_t) -> Box<wasm_tagtype_t> {
    Box::new(wasm_tagtype_t::new(ty.ty.tag_type()))
}
