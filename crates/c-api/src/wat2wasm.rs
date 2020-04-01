use crate::wasm_byte_vec_t;

#[no_mangle]
pub extern "C" fn wasmtime_wat2wasm(
    wat: &wasm_byte_vec_t,
    ret: &mut wasm_byte_vec_t,
    error: Option<&mut wasm_byte_vec_t>,
) -> bool {
    let wat = match std::str::from_utf8(wat.as_slice()) {
        Ok(s) => s,
        Err(_) => {
            if let Some(error) = error {
                error.set_buffer(b"input was not valid utf-8".to_vec());
            }
            return false;
        }
    };
    match wat::parse_str(wat) {
        Ok(bytes) => {
            ret.set_buffer(bytes.into());
            true
        }
        Err(e) => {
            if let Some(error) = error {
                error.set_buffer(e.to_string().into_bytes());
            }
            false
        }
    }
}
