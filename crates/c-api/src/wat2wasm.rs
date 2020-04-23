use crate::{bad_utf8, handle_result, wasm_byte_vec_t, wasmtime_error_t};

#[no_mangle]
pub extern "C" fn wasmtime_wat2wasm(
    wat: &wasm_byte_vec_t,
    ret: &mut wasm_byte_vec_t,
) -> Option<Box<wasmtime_error_t>> {
    let wat = match std::str::from_utf8(wat.as_slice()) {
        Ok(s) => s,
        Err(_) => return bad_utf8(),
    };
    handle_result(wat::parse_str(wat).map_err(|e| e.into()), |bytes| {
        ret.set_buffer(bytes)
    })
}
