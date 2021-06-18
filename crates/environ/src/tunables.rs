use serde::{Deserialize, Serialize};

/// Tunable parameters for WebAssembly compilation.
#[derive(Clone, Hash, Serialize, Deserialize)]
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

    /// Whether or not fuel is enabled for generated code, meaning that fuel
    /// will be consumed every time a wasm instruction is executed.
    pub consume_fuel: bool,

    /// Whether or not to treat the static memory bound as the maximum for unbounded heaps.
    pub static_memory_bound_is_maximum: bool,

    /// Whether or not linear memory allocations will have a guard region at the
    /// beginning of the allocation in addition to the end.
    pub guard_before_linear_memory: bool,
}

impl Default for Tunables {
    fn default() -> Self {
        Self {
            // 64-bit has tons of address space to static memories can have 4gb
            // address space reservations liberally by default, allowing us to
            // help eliminate bounds checks.
            //
            // Coupled with a 2 GiB address space guard it lets us translate
            // wasm offsets into x86 offsets as aggressively as we can.
            #[cfg(target_pointer_width = "64")]
            static_memory_bound: 0x1_0000,
            #[cfg(target_pointer_width = "64")]
            static_memory_offset_guard_size: 0x8000_0000,

            // For 32-bit we scale way down to 10MB of reserved memory. This
            // impacts performance severely but allows us to have more than a
            // few instances running around.
            #[cfg(target_pointer_width = "32")]
            static_memory_bound: (10 * (1 << 20)) / crate::WASM_PAGE_SIZE,
            #[cfg(target_pointer_width = "32")]
            static_memory_offset_guard_size: 0x1_0000,

            // Size in bytes of the offset guard for dynamic memories.
            //
            // Allocate a small guard to optimize common cases but without
            // wasting too much memory.
            dynamic_memory_offset_guard_size: 0x1_0000,

            generate_native_debuginfo: false,
            parse_wasm_debuginfo: true,
            interruptable: false,
            consume_fuel: false,
            static_memory_bound_is_maximum: false,
            guard_before_linear_memory: true,
        }
    }
}
