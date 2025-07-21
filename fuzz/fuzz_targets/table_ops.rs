#![no_main]

use libfuzzer_sys::{fuzz_mutator, fuzz_target, fuzzer_mutate};
use mutatis::Session;
use wasmtime_fuzzing::generators::table_ops::TableOps;
use wasmtime_fuzzing::oracles::fuzz_table_ops;

fuzz_target!(|input: (u64, TableOps)| {
    fuzz_table_ops(input);
});

fuzz_mutator!(|data: &mut [u8], size: usize, max_size: usize, seed: u32| {
    let _ = env_logger::try_init();

    // With probability of about 1/8, just use the default mutator.
    if seed.count_ones() % 8 == 0 {
        return fuzzer_mutate(data, size, max_size);
    }

    // Fallback to default on decode failure
    let mut tuple =
        bincode::decode_from_slice::<(u64, TableOps), _>(data, bincode::config::standard())
            .map_or_else(|_err| (0, TableOps::default()), |(tuple, _)| tuple);

    let mut session = Session::new().seed(seed.into()).shrink(max_size < size);

    if session.mutate(&mut tuple).is_ok() {
        // Re-encode the mutated ops back into `data`.
        loop {
            if let Ok(new_size) =
                bincode::encode_into_slice(&tuple, data, bincode::config::standard())
            {
                return new_size;
            }
            // When re-encoding fails (presumably because `data` is not
            // large enough) then pop an op off the end and try again.
            if tuple.1.pop() {
                continue;
            }
            break;
        }
    }
    // Fall back to the fuzzer's default mutation strategies.
    fuzzer_mutate(data, size, max_size)
});
