use crate::borrow::BorrowChecker;
use crate::{BorrowHandle, GuestError, GuestMemory, Region};
use std::cell::UnsafeCell;

/// Lightweight `wasmtime::Memory` wrapper so we can implement the
/// `wiggle::GuestMemory` trait on it.
pub struct WasmtimeGuestMemory<'a> {
    mem: &'a [UnsafeCell<u8>],
    bc: BorrowChecker,
    shared: bool,
}

// These need to be reapplied due to the usage of `UnsafeCell` internally.
unsafe impl Send for WasmtimeGuestMemory<'_> {}
unsafe impl Sync for WasmtimeGuestMemory<'_> {}

impl<'a> WasmtimeGuestMemory<'a> {
    pub fn new(mem: &'a mut [u8]) -> Self {
        Self {
            // SAFETY: here the `&mut [u8]` is casted to `&[UnsafeCell<u8>]`
            // which is losing in effect the `&mut` access but retaining the
            // borrow. This is done to reflect how the memory is not safe to
            // access while multiple borrows are handed out internally, checked
            // with `bc` below.
            //
            // Additionally this allows unshared memories to have the same
            // internal representation as shared memories.
            mem: unsafe { std::slice::from_raw_parts(mem.as_ptr().cast(), mem.len()) },

            // Wiggle does not expose any methods for functions to re-enter
            // the WebAssembly instance, or expose the memory via non-wiggle
            // mechanisms. However, the user-defined code may end up
            // re-entering the instance, in which case this is an incorrect
            // implementation - we require exactly one BorrowChecker exist per
            // instance.
            // This BorrowChecker construction is a holdover until it is
            // integrated fully with wasmtime:
            // https://github.com/bytecodealliance/wasmtime/issues/1917
            bc: BorrowChecker::new(),
            shared: false,
        }
    }

    pub fn shared(mem: &'a [UnsafeCell<u8>]) -> Self {
        Self {
            mem,
            bc: BorrowChecker::new(),
            shared: true,
        }
    }
}

unsafe impl GuestMemory for WasmtimeGuestMemory<'_> {
    fn base(&self) -> &[UnsafeCell<u8>] {
        self.mem
    }
    fn has_outstanding_borrows(&self) -> bool {
        self.bc.has_outstanding_borrows()
    }
    fn is_shared_borrowed(&self, r: Region) -> bool {
        self.bc.is_shared_borrowed(r)
    }
    fn is_mut_borrowed(&self, r: Region) -> bool {
        self.bc.is_mut_borrowed(r)
    }
    fn shared_borrow(&self, r: Region) -> Result<BorrowHandle, GuestError> {
        self.bc.shared_borrow(r)
    }
    fn mut_borrow(&self, r: Region) -> Result<BorrowHandle, GuestError> {
        self.bc.mut_borrow(r)
    }
    fn shared_unborrow(&self, h: BorrowHandle) {
        self.bc.shared_unborrow(h)
    }
    fn mut_unborrow(&self, h: BorrowHandle) {
        self.bc.mut_unborrow(h)
    }
    fn is_shared_memory(&self) -> bool {
        self.shared
    }
}
