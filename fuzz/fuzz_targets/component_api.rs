#![no_main]
#![allow(dead_code, reason = "fuzz-generation sometimes generates unused types")]

use libfuzzer_sys::{arbitrary, fuzz_target};
use wasmtime_fuzzing::oracles;

include!(concat!(env!("OUT_DIR"), "/static_component_api.rs"));

fn target(input: &mut arbitrary::Unstructured) -> arbitrary::Result<()> {
    if input.arbitrary()? {
        static_component_api_target(input)
    } else {
        oracles::component_api::dynamic_component_api_target(input)
    }
}

fuzz_target!(|bytes: &[u8]| {
    let _ = target(&mut arbitrary::Unstructured::new(bytes));
});
