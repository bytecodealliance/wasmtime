use anyhow::{anyhow, bail, Result};
use serde_derive::{Deserialize, Serialize};
use target_lexicon::{PointerWidth, Triple};

/// Tunable parameters for WebAssembly compilation.
#[derive(Clone, Hash, Serialize, Deserialize, Debug)]
pub struct Tunables {
    /// For static heaps, the size in wasm pages of the heap protected by bounds
    /// checking.
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

    /// Whether or not to treat the static memory bound as the maximum for
    /// unbounded heaps.
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

    /// Whether or not lowerings for relaxed simd instructions are forced to
    /// be deterministic.
    pub relaxed_simd_deterministic: bool,

    /// Whether or not Wasm functions can be tail-called or not.
    pub tail_callable: bool,

    /// Whether or not Wasm functions target the winch abi.
    pub winch_callable: bool,

    /// Whether we implement a one-entry cache at each call_indirect
    /// site.
    pub cache_call_indirects: bool,
}

impl Tunables {
    /// Returns a `Tunables` configuration assumed for running code on the host.
    pub fn default_host() -> Self {
        if cfg!(miri) {
            Tunables::default_miri()
        } else if cfg!(target_pointer_width = "32") {
            Tunables::default_u32()
        } else if cfg!(target_pointer_width = "64") {
            Tunables::default_u64()
        } else {
            panic!("unsupported target_pointer_width");
        }
    }

    /// Returns the default set of tunables for the given target triple.
    pub fn default_for_target(target: &Triple) -> Result<Self> {
        match target
            .pointer_width()
            .map_err(|_| anyhow!("failed to retrieve target pointer width"))?
        {
            PointerWidth::U32 => Ok(Tunables::default_u32()),
            PointerWidth::U64 => Ok(Tunables::default_u64()),
            _ => bail!("unsupported target pointer width"),
        }
    }

    /// Returns the default set of tunables for running under MIRI.
    pub fn default_miri() -> Tunables {
        Tunables {
            // No virtual memory tricks are available on miri so make these
            // limits quite conservative.
            static_memory_bound: (1 << 20) / crate::WASM_PAGE_SIZE as u64,
            static_memory_offset_guard_size: 0,
            dynamic_memory_offset_guard_size: 0,
            dynamic_memory_growth_reserve: 0,

            // General options which have the same defaults regardless of
            // architecture.
            generate_native_debuginfo: false,
            parse_wasm_debuginfo: true,
            consume_fuel: false,
            epoch_interruption: false,
            static_memory_bound_is_maximum: false,
            guard_before_linear_memory: true,
            generate_address_map: true,
            debug_adapter_modules: false,
            relaxed_simd_deterministic: false,
            tail_callable: false,
            winch_callable: false,
            cache_call_indirects: false,
        }
    }

    /// Returns the default set of tunables for running under a 32-bit host.
    pub fn default_u32() -> Tunables {
        Tunables {
            // For 32-bit we scale way down to 10MB of reserved memory. This
            // impacts performance severely but allows us to have more than a
            // few instances running around.
            static_memory_bound: (10 * (1 << 20)) / crate::WASM_PAGE_SIZE as u64,
            static_memory_offset_guard_size: 0x1_0000,
            dynamic_memory_offset_guard_size: 0x1_0000,
            dynamic_memory_growth_reserve: 1 << 20, // 1MB

            ..Tunables::default_miri()
        }
    }

    /// Returns the default set of tunables for running under a 64-bit host.
    pub fn default_u64() -> Tunables {
        Tunables {
            // 64-bit has tons of address space to static memories can have 4gb
            // address space reservations liberally by default, allowing us to
            // help eliminate bounds checks.
            //
            // Coupled with a 2 GiB address space guard it lets us translate
            // wasm offsets into x86 offsets as aggressively as we can.
            static_memory_bound: 0x1_0000,
            static_memory_offset_guard_size: 0x8000_0000,

            // Size in bytes of the offset guard for dynamic memories.
            //
            // Allocate a small guard to optimize common cases but without
            // wasting too much memory.
            dynamic_memory_offset_guard_size: 0x1_0000,

            // We've got lots of address space on 64-bit so use a larger
            // grow-into-this area, but on 32-bit we aren't as lucky. Miri is
            // not exactly fast so reduce memory consumption instead of trying
            // to avoid memory movement.
            dynamic_memory_growth_reserve: 2 << 30, // 2GB

            ..Tunables::default_miri()
        }
    }
}
