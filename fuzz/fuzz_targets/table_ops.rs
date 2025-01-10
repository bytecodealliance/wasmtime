#![no_main]

use libfuzzer_sys::fuzz_target;
use wasmtime_fuzzing::generators::{Config, table_ops::TableOps};

fuzz_target!(|pair: (Config, TableOps)| {
    let (config, ops) = pair;
    let _ = wasmtime_fuzzing::oracles::table_ops(config, ops);
});
