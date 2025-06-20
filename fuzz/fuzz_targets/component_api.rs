#![no_main]

use libfuzzer_sys::{arbitrary, fuzz_target};
use wasmtime_fuzzing::oracles;

include!(concat!(env!("OUT_DIR"), "/static_component_api.rs"));

fn target(input: &mut arbitrary::Unstructured) -> arbitrary::Result<()> {
    if input.arbitrary()? {
        static_component_api_target(input)
    } else {
        oracles::dynamic_component_api_target(input)
    }
}

fuzz_target!(|bytes: &[u8]| {
    match target(&mut arbitrary::Unstructured::new(bytes)) {
        Ok(()) | Err(arbitrary::Error::NotEnoughData) => (),
        Err(error) => panic!("{}", error),
    }
});
