use serde::{Deserialize, Serialize};

/// Tunable parameters for WebAssembly compilation.
#[derive(Clone, Hash, Serialize, Deserialize)]
pub struct Tunables {
    /// For static heaps, the size in wasm pages of the heap protected by bounds checking.
    pub static_memory_bound: u64,

    /// The size in bytes of the offset guard for static heaps.
    pub static_memory_offset_guard_size: u64,

    /// The size in bytes of the offset guard for dynamic heaps.
    pub dynamic_memory_offset_guard_size: u64,

    /// The size, in bytes, of reserved memory at the end of a "dynamic" memory,
    /// before the guard page, that memory can grow into. This is intended to
    /// amortize the cost of `memory.grow` in the same manner that `Vec<T>` has
    /// space not in use to grow into.
    pub dynamic_memory_growth_reserve: u64,

    /// Whether or not to generate native DWARF debug information.
    pub generate_native_debuginfo: bool,

    /// Whether or not to retain DWARF sections in compiled modules.
    pub parse_wasm_debuginfo: bool,

    /// Whether or not fuel is enabled for generated code, meaning that fuel
    /// will be consumed every time a wasm instruction is executed.
    pub consume_fuel: bool,

    /// Whether or not we use epoch-based interruption.
    pub epoch_interruption: bool,

    /// Whether or not to treat the static memory bound as the maximum for unbounded heaps.
    pub static_memory_bound_is_maximum: bool,

    /// Whether or not linear memory allocations will have a guard region at the
    /// beginning of the allocation in addition to the end.
    pub guard_before_linear_memory: bool,

    /// Indicates whether an address map from compiled native code back to wasm
    /// offsets in the original file is generated.
    pub generate_address_map: bool,

    /// Flag for the component module whether adapter modules have debug
    /// assertions baked into them.
    pub debug_adapter_modules: bool,
}

impl Default for Tunables {
    fn default() -> Self {
        let (static_memory_bound, static_memory_offset_guard_size) =
            if cfg!(target_pointer_width = "64") {
                // 64-bit has tons of address space to static memories can have 4gb
                // address space reservations liberally by default, allowing us to
                // help eliminate bounds checks.
                //
                // Coupled with a 2 GiB address space guard it lets us translate
                // wasm offsets into x86 offsets as aggressively as we can.
                (0x1_0000, 0x8000_0000)
            } else if cfg!(target_pointer_width = "32") {
                // For 32-bit we scale way down to 10MB of reserved memory. This
                // impacts performance severely but allows us to have more than a
                // few instances running around.
                ((10 * (1 << 20)) / crate::WASM_PAGE_SIZE as u64, 0x1_0000)
            } else {
                panic!("unsupported target_pointer_width");
            };
        Self {
            static_memory_bound,
            static_memory_offset_guard_size,

            // Size in bytes of the offset guard for dynamic memories.
            //
            // Allocate a small guard to optimize common cases but without
            // wasting too much memory.
            dynamic_memory_offset_guard_size: 0x1_0000,

            // We've got lots of address space on 64-bit so use a larger
            // grow-into-this area, but on 32-bit we aren't as lucky.
            #[cfg(target_pointer_width = "64")]
            dynamic_memory_growth_reserve: 2 << 30, // 2GB
            #[cfg(target_pointer_width = "32")]
            dynamic_memory_growth_reserve: 1 << 20, // 1MB

            generate_native_debuginfo: false,
            parse_wasm_debuginfo: true,
            consume_fuel: false,
            epoch_interruption: false,
            static_memory_bound_is_maximum: false,
            guard_before_linear_memory: true,
            generate_address_map: true,
            debug_adapter_modules: false,
        }
    }
}
