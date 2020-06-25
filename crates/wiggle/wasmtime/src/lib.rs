pub use wasmtime_wiggle_macro::*;
pub use wiggle::*;

/// Lightweight `wasmtime::Memory` wrapper so we can implement the
/// `wiggle::GuestMemory` trait on it.
pub struct WasmtimeGuestMemory {
    mem: wasmtime::Memory,
    bc: BorrowChecker,
}

impl WasmtimeGuestMemory {
    pub fn new(mem: wasmtime::Memory, bc: BorrowChecker) -> Self {
        Self { mem, bc }
    }
}

unsafe impl GuestMemory for WasmtimeGuestMemory {
    fn base(&self) -> (*mut u8, u32) {
        (self.mem.data_ptr(), self.mem.data_size() as _)
    }
    fn borrow_checker(&self) -> &BorrowChecker {
        &self.bc
    }
}
