//! Custom fuzz input mutators.
//!
//! The functions in this module are intended to be used with [the
//! `libfuzzer_sys::fuzz_mutator!` macro][fuzz-mutator].
//!
//! [fuzz-mutator]: https://docs.rs/libfuzzer-sys/latest/libfuzzer_sys/macro.fuzz_mutator.html

use arbitrary::{Arbitrary, Unstructured};
use std::sync::Arc;

/// Use [`wasm-mutate`][wasm-mutate] to mutate a fuzz input.
///
/// [wasm-mutate]: https://github.com/bytecodealliance/wasm-tools/tree/main/crates/wasm-mutate
pub fn wasm_mutate(
    data: &mut [u8],
    size: usize,
    max_size: usize,
    seed: u32,
    libfuzzer_mutate: fn(data: &mut [u8], size: usize, max_size: usize) -> usize,
) -> usize {
    const MUTATION_FUEL: u64 = 100;
    const MUTATION_ITERS: usize = 100;

    let wasm = &data[..size];

    if wasmparser::validate(wasm).is_ok() {
        let mut wasm_mutate = wasm_mutate::WasmMutate::default();
        wasm_mutate
            .seed(seed.into())
            .fuel(MUTATION_FUEL)
            .reduce(max_size < size)
            .raw_mutate_func(Some(Arc::new(move |data, max_size| {
                let len = data.len();

                // The given max could be very large, so clamp it to no more
                // than `len * 2` in any single, given mutation. This way we
                // don't over-allocate a bunch of space.
                let max_size = std::cmp::min(max_size, len * 2);
                // Also, the max must always be greater than zero (`libfuzzer`
                // asserts this).
                let max_size = std::cmp::max(max_size, 1);

                // Make sure we have capacity in case `libfuzzer` decides to
                // grow this data.
                if max_size > len {
                    data.resize(max_size, 0);
                }

                // Finally, have `libfuzzer` mutate the data!
                let new_len = libfuzzer_mutate(data, len, max_size);

                // Resize the data to the mutated size, releasing any extra
                // capacity that we don't need anymore.
                data.resize(new_len, 0);
                data.shrink_to_fit();

                Ok(())
            })));

        let wasm = wasm.to_vec();
        let mutations = wasm_mutate.run(&wasm);
        if let Ok(mutations) = mutations {
            for mutation in mutations.take(MUTATION_ITERS) {
                if let Ok(mutated_wasm) = mutation {
                    if mutated_wasm.len() <= max_size {
                        data[..mutated_wasm.len()].copy_from_slice(&mutated_wasm);
                        return mutated_wasm.len();
                    }
                }
            }
        }
    }

    // If we can't mutate the input because it isn't valid Wasm or `wasm-mutate`
    // otherwise fails, try to use `wasm-smith` to generate a new, arbitrary
    // Wasm module that fits within the max-size limit.
    let mut u = Unstructured::new(&data[..max_size]);
    if let Ok(module) = wasm_smith::Module::arbitrary(&mut u) {
        let wasm = module.to_bytes();
        if wasm.len() <= max_size {
            data[..wasm.len()].copy_from_slice(&wasm);
            return wasm.len();
        }
    }

    // Otherwise, try to return an empty Wasm module:
    //
    // ```
    // (module)
    // ```
    static EMPTY_WASM: &[u8] = &[0x00, b'a', b's', b'm', 0x01, 0x00, 0x00, 0x00];
    if EMPTY_WASM.len() <= max_size {
        data[..EMPTY_WASM.len()].copy_from_slice(EMPTY_WASM);
        return EMPTY_WASM.len();
    }

    // If the max size is even smaller than an empty Wasm module, then just let
    // `libfuzzer` mutate the data.
    libfuzzer_mutate(data, size, max_size)
}
