//! Fuzzing infrastructure for Wasmtime.

#![deny(missing_docs)]

pub use wasm_mutate;
pub use wasm_smith;
pub mod generators;
pub mod mutators;
pub mod oracles;

/// One time start up initialization for fuzzing:
///
/// * Enables `env_logger`.
///
/// * Restricts `rayon` to a single thread in its thread pool, for more
///   deterministic executions.
///
/// If a fuzz target is taking raw input bytes from the fuzzer, it is fine to
/// call this function in the fuzz target's oracle or in the fuzz target
/// itself. However, if the fuzz target takes an `Arbitrary` type, and the
/// `Arbitrary` implementation is not derived and does interesting things, then
/// the `Arbitrary` implementation should call this function, since it runs
/// before the fuzz target itself.
pub(crate) fn init_fuzzing() {
    static INIT: std::sync::Once = std::sync::Once::new();

    INIT.call_once(|| {
        let _ = env_logger::try_init();

        let _ = rayon::ThreadPoolBuilder::new()
            .num_threads(1)
            .build_global();
    })
}
