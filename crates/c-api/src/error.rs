use crate::{wasm_name_t, wasm_trap_t};
use anyhow::{anyhow, Error, Result};
use wasmtime::Trap;

#[repr(C)]
pub struct wasmtime_error_t {
    error: Error,
}

wasmtime_c_api_macros::declare_own!(wasmtime_error_t);

impl wasmtime_error_t {
    pub(crate) fn to_trap(self) -> Box<wasm_trap_t> {
        Box::new(wasm_trap_t::new(Trap::from(self.error)))
    }
}

impl From<Error> for wasmtime_error_t {
    fn from(error: Error) -> wasmtime_error_t {
        wasmtime_error_t { error }
    }
}

pub(crate) fn handle_result<T>(
    result: Result<T>,
    ok: impl FnOnce(T),
) -> Option<Box<wasmtime_error_t>> {
    match result {
        Ok(value) => {
            ok(value);
            None
        }
        Err(error) => Some(Box::new(wasmtime_error_t { error })),
    }
}

pub(crate) fn bad_utf8() -> Option<Box<wasmtime_error_t>> {
    Some(Box::new(wasmtime_error_t {
        error: anyhow!("input was not valid utf-8"),
    }))
}

#[no_mangle]
pub extern "C" fn wasmtime_error_message(error: &wasmtime_error_t, message: &mut wasm_name_t) {
    message.set_buffer(format!("{:?}", error.error).into_bytes());
}
