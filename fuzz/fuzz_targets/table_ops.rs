#![no_main]

use libfuzzer_sys::fuzz_target;
use wasmtime_fuzzing::generators::{table_ops::TableOps, Config};

fuzz_target!(|pair: (Config, TableOps)| {
    let (config, ops) = pair;
    wasmtime_fuzzing::oracles::table_ops(config, ops);
});
