/// Tunable parameters for WebAssembly compilation.
#[derive(Clone, Hash)]
pub struct Tunables {
    /// For static heaps, the size in wasm pages of the heap protected by bounds checking.
    pub static_memory_bound: u32,

    /// The size in bytes of the offset guard for static heaps.
    pub static_memory_offset_guard_size: u64,

    /// The size in bytes of the offset guard for dynamic heaps.
    pub dynamic_memory_offset_guard_size: u64,

    /// Whether or not to generate native DWARF debug information.
    pub generate_native_debuginfo: bool,

    /// Whether or not to retain DWARF sections in compiled modules.
    pub parse_wasm_debuginfo: bool,

    /// Whether or not to enable the ability to interrupt wasm code dynamically.
    ///
    /// More info can be found about the implementation in
    /// crates/environ/src/cranelift.rs. Note that you can't interrupt host
    /// calls and interrupts are implemented through the `VMInterrupts`
    /// structure, or `InterruptHandle` in the `wasmtime` crate.
    pub interruptable: bool,
}

impl Default for Tunables {
    fn default() -> Self {
        Self {
            #[cfg(target_pointer_width = "32")]
            /// Size in wasm pages of the bound for static memories.
            static_memory_bound: 0x4000,
            #[cfg(target_pointer_width = "64")]
            /// Size in wasm pages of the bound for static memories.
            ///
            /// When we allocate 4 GiB of address space, we can avoid the
            /// need for explicit bounds checks.
            static_memory_bound: 0x1_0000,

            #[cfg(target_pointer_width = "32")]
            /// Size in bytes of the offset guard for static memories.
            static_memory_offset_guard_size: 0x1_0000,
            #[cfg(target_pointer_width = "64")]
            /// Size in bytes of the offset guard for static memories.
            ///
            /// Allocating 2 GiB of address space lets us translate wasm
            /// offsets into x86 offsets as aggressively as we can.
            static_memory_offset_guard_size: 0x8000_0000,

            /// Size in bytes of the offset guard for dynamic memories.
            ///
            /// Allocate a small guard to optimize common cases but without
            /// wasting too much memory.
            dynamic_memory_offset_guard_size: 0x1_0000,

            generate_native_debuginfo: false,
            parse_wasm_debuginfo: true,
            interruptable: false,
        }
    }
}
