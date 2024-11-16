#![no_main]
#![expect(clippy::allow_attributes_without_reason, reason = "crate not migrated")]

use libfuzzer_sys::{arbitrary, fuzz_target};
use wasmtime_fuzzing::oracles;

include!(concat!(env!("OUT_DIR"), "/static_component_api.rs"));

#[allow(unused_imports)]
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
