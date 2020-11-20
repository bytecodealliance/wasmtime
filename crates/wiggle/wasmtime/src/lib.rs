pub use wasmtime_wiggle_macro::*;
pub use wiggle::*;

use wiggle_borrow::BorrowChecker;

/// Lightweight `wasmtime::Memory` wrapper so we can implement the
/// `wiggle::GuestMemory` trait on it.
pub struct WasmtimeGuestMemory {
    mem: wasmtime::Memory,
    bc: BorrowChecker,
}

impl WasmtimeGuestMemory {
    pub fn new(mem: wasmtime::Memory) -> Self {
        Self {
            mem,
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
        }
    }
}

unsafe impl GuestMemory for WasmtimeGuestMemory {
    fn base(&self) -> (*mut u8, u32) {
        (self.mem.data_ptr(), self.mem.data_size() as _)
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
}
