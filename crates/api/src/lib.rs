//! Wasmtime's embedding API
//!
//! This crate contains a high-level API used to interact with WebAssembly
//! modules. The API here is intended to mirror the proposed [WebAssembly C
//! API](https://github.com/WebAssembly/wasm-c-api), with small extensions here
//! and there to implement Rust idioms. This crate also defines the actual C API
//! itself for consumption from other languages.

#![deny(missing_docs, intra_doc_link_resolution_failure)]
#![doc(test(attr(deny(warnings))))]
#![doc(test(attr(allow(dead_code, unused_variables, unused_mut))))]

mod externals;
mod frame_info;
mod func;
mod instance;
mod linker;
mod module;
mod r#ref;
mod runtime;
mod trampoline;
mod trap;
mod types;
mod values;

pub use crate::externals::*;
pub use crate::frame_info::FrameInfo;
pub use crate::func::*;
pub use crate::instance::Instance;
pub use crate::linker::*;
pub use crate::module::Module;
pub use crate::r#ref::{AnyRef, HostRef};
pub use crate::runtime::*;
pub use crate::trap::Trap;
pub use crate::types::*;
pub use crate::values::*;

cfg_if::cfg_if! {
    if #[cfg(unix)] {
        pub mod unix;
    } else if #[cfg(windows)] {
        pub mod windows;
    } else {
        // ... unknown os!
    }
}

/// Debug helpers.
pub mod debug_builtins {
    use wasmtime_runtime::{InstanceHandle, VMContext};

    static mut VMCTX: *mut VMContext = std::ptr::null_mut();

    #[no_mangle]
    #[allow(dead_code, missing_docs)]
    pub unsafe extern "C" fn resolve_vmctx_memory_ptr(p: *const u32) -> *const u8 {
        let handle = InstanceHandle::from_vmctx(VMCTX);
        let mem = if let Some(wasmtime_runtime::Export::Memory(mem)) = handle.lookup("memory") {
            mem.definition
        } else {
            panic!();
        };
        let ptr = std::ptr::read(p);
        (*mem).base.add(ptr as usize)
    }

    #[no_mangle]
    #[allow(dead_code, missing_docs)]
    pub unsafe extern "C" fn set_vmctx_memory(vmctx_ptr: *mut VMContext) {
        VMCTX = vmctx_ptr;
    }

    /// Ensures that _set_vmctx_memory and _resolve_vmctx_memory_ptr are linked and
    /// exported as symbols. It is a workaround: the executable normally ignores
    /// `pub extern "C"`, see rust-lang/rust#25057.
    pub fn ensure_exported() {
        unsafe {
            std::ptr::read_volatile(resolve_vmctx_memory_ptr as *const u8);
            std::ptr::read_volatile(set_vmctx_memory as *const u8);
        }
    }
}
