#![no_main]

use libfuzzer_sys::arbitrary::{Arbitrary, Unstructured};
use libfuzzer_sys::{fuzz_mutator, fuzz_target, fuzzer_mutate};
use mutatis::Session;
use postcard::{from_bytes, to_slice};
use rand::{Rng, SeedableRng};
use wasmtime_fuzzing::generators::table_ops::TableOps;
use wasmtime_fuzzing::oracles::table_ops;

fuzz_target!(|data: &[u8]| {
    let Ok((seed, ops)) = postcard::from_bytes::<(u64, TableOps)>(data) else {
        return;
    };

    let mut buf = [0u8; 1024];
    let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
    rng.fill(&mut buf);

    let u = Unstructured::new(&buf);
    let Ok(config) = wasmtime_fuzzing::generators::Config::arbitrary_take_rest(u) else {
        return;
    };

    let _ = table_ops(config, ops);
});

fuzz_mutator!(|data: &mut [u8], size: usize, max_size: usize, seed: u32| {
    let _ = env_logger::try_init();

    // With probability of about 1/8, use default mutator
    if seed.count_ones() % 8 == 0 {
        return fuzzer_mutate(data, size, max_size);
    }

    // Try to decode using postcard; fallback to default input on failure
    let mut tuple: (u64, TableOps) = from_bytes(&data[..size]).ok().unwrap_or_default();

    let mut session = Session::new().seed(seed.into()).shrink(max_size < size);

    if session.mutate(&mut tuple).is_ok() {
        loop {
            if let Ok(encoded) = to_slice(&tuple, data) {
                return encoded.len();
            }

            // Attempt to shrink ops if encoding fails (e.g., buffer too small)
            if tuple.1.pop() {
                continue;
            }

            break;
        }
    }

    // Fallback to default libfuzzer mutator
    fuzzer_mutate(data, size, max_size)
});
