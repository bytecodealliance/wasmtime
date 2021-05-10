#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals)]
// #![allow(unknown_lints)]
// #![allow(improper_ctypes_definitions)]

mod config;
mod engine;
mod error;
mod r#extern;
mod func;
mod global;
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

// #[cfg(feature = "wasi")]
// mod wasi;
// #[cfg(feature = "wasi")]
// pub use crate::wasi::*;

#[cfg(feature = "wat")]
mod wat2wasm;
#[cfg(feature = "wat")]
pub use crate::wat2wasm::*;

/// Initialize a `MaybeUninit<T>`
///
/// TODO: Replace calls to this function with
/// https://doc.rust-lang.org/nightly/std/mem/union.MaybeUninit.html#method.write
/// once it is stable.
pub(crate) fn initialize<T>(dst: &mut std::mem::MaybeUninit<T>, val: T) {
    unsafe {
        std::ptr::write(dst.as_mut_ptr(), val);
    }
}

pub struct ForeignData {
    data: *mut std::ffi::c_void,
    finalizer: Option<extern "C" fn(*mut std::ffi::c_void)>,
}

unsafe impl Send for ForeignData {}
unsafe impl Sync for ForeignData {}

impl Drop for ForeignData {
    fn drop(&mut self) {
        if let Some(f) = self.finalizer {
            f(self.data);
        }
    }
}
