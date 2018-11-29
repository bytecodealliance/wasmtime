/// Tunable parameters for WebAssembly compilation.
#[derive(Clone)]
pub struct Tunables {
    /// For static heaps, the size of the heap protected by bounds checking.
    pub static_memory_bound: u32,

    /// The size of the offset guard.
    pub offset_guard_size: u64,
}

impl Default for Tunables {
    fn default() -> Self {
        Self {
            /// Size in wasm pages of the bound for static memories.
            ///
            /// When we allocate 4 GiB of address space, we can avoid the
            /// need for explicit bounds checks.
            static_memory_bound: 0x1_0000,

            /// Size in bytes of the offset guard.
            ///
            /// Allocating 2 GiB of address space lets us translate wasm
            /// offsets into x86 offsets as aggressively as we can.
            offset_guard_size: 0x8000_0000,
        }
    }
}
