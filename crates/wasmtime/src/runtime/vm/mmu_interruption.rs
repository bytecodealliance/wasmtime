//! Support machinery for the mmu-interruption feature

/// Whether this build of Wasmtime can run code with
/// [`Config::mmu_interruption`](crate::Config::mmu_interruption) enabled on
/// this host. Exposed as a const so other crates can consult it.
pub const HAS_MMU_INTERRUPTION_RUNTIME: bool = cfg!(has_mmu_interruption_runtime);
