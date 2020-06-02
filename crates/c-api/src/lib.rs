//! This file defines the extern "C" API, which is compatible with the
//! [Wasm C API](https://github.com/WebAssembly/wasm-c-api).

#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals)]

// TODO complete the C API

mod config;
mod engine;
mod error;
mod r#extern;
mod func;
mod global;
mod host_ref;
mod instance;
mod linker;
mod memory;
mod module;
mod r#ref;
mod store;
mod table;
mod trap;
mod types;
mod val;
mod vec;

pub use crate::config::*;
pub use crate::engine::*;
pub use crate::error::*;
pub use crate::func::*;
pub use crate::global::*;
pub use crate::instance::*;
pub use crate::linker::*;
pub use crate::memory::*;
pub use crate::module::*;
pub use crate::r#extern::*;
pub use crate::r#ref::*;
pub use crate::store::*;
pub use crate::table::*;
pub use crate::trap::*;
pub use crate::types::*;
pub use crate::val::*;
pub use crate::vec::*;

#[cfg(feature = "wasi")]
mod wasi;
#[cfg(feature = "wasi")]
pub use crate::wasi::*;

#[cfg(feature = "wat")]
mod wat2wasm;
#[cfg(feature = "wat")]
pub use crate::wat2wasm::*;

#[repr(C)]
#[derive(Clone)]
pub struct wasm_foreign_t {
    _unused: [u8; 0],
}

#[repr(C)]
#[derive(Clone)]
pub struct wasm_shared_module_t {
    _unused: [u8; 0],
}

struct HostInfoState {
    info: *mut std::ffi::c_void,
    finalizer: Option<extern "C" fn(arg1: *mut std::ffi::c_void)>,
}

impl Drop for HostInfoState {
    fn drop(&mut self) {
        if let Some(f) = &self.finalizer {
            f(self.info);
        }
    }
}
