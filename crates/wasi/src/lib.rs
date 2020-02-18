pub mod old;

use wasi_common::hostcalls;

pub use wasi_common::{WasiCtx, WasiCtxBuilder};

// Defines a `struct Wasi` with member fields and appropriate APIs for dealing
// with all the various WASI exports.
wig::define_wasi_struct!(
    "snapshot" "wasi_snapshot_preview1"
);

pub fn is_wasi_module(name: &str) -> bool {
    // FIXME: this should be more conservative, but while WASI is in flux and
    // we're figuring out how to support multiple revisions, this should do the
    // trick.
    name.starts_with("wasi")
}

/// This is an internal structure used to acquire a handle on the caller's
/// wasm memory buffer.
///
/// This exploits how we can implement `WasmTy` for ourselves locally even
/// though crates in general should not be doing that. This is a crate in
/// the wasmtime project, however, so we should be able to keep up with our own
/// changes.
///
/// In general this type is wildly unsafe. We need to update the wasi crates to
/// probably work with more `wasmtime`-like APIs to grip with the unsafety
/// around dealing with caller memory.
struct WasiCallerMemory {
    base: *mut u8,
    len: usize,
}

impl wasmtime::WasmTy for WasiCallerMemory {
    type Abi = ();

    fn push(_dst: &mut Vec<wasmtime::ValType>) {}

    fn matches(_tys: impl Iterator<Item = wasmtime::ValType>) -> bool {
        true
    }

    fn from_abi(vmctx: *mut wasmtime_runtime::VMContext, _abi: ()) -> Self {
        unsafe {
            match wasmtime_runtime::InstanceHandle::from_vmctx(vmctx).lookup("memory") {
                Some(wasmtime_runtime::Export::Memory {
                    definition,
                    vmctx: _,
                    memory: _,
                }) => WasiCallerMemory {
                    base: (*definition).base,
                    len: (*definition).current_length,
                },
                _ => WasiCallerMemory {
                    base: std::ptr::null_mut(),
                    len: 0,
                },
            }
        }
    }

    fn into_abi(self) {}
}

impl WasiCallerMemory {
    unsafe fn get(&self) -> Result<&mut [u8], wasi_common::wasi::__wasi_errno_t> {
        if self.base.is_null() {
            Err(wasi_common::wasi::__WASI_ERRNO_INVAL)
        } else {
            Ok(std::slice::from_raw_parts_mut(self.base, self.len))
        }
    }
}
