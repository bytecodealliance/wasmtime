#![no_main]

use cranelift_codegen::{isa, settings};
use cranelift_wasm::{translate_module, DummyEnvironment, ReturnMode};
use libfuzzer_sys::fuzz_target;
use std::str::FromStr;
use target_lexicon::triple;
use wasmtime_fuzzing::generators;

fuzz_target!(|data: generators::WasmOptTtf| {
    let flags = settings::Flags::new(settings::builder());
    let triple = triple!("x86_64");
    let isa = isa::lookup(triple).unwrap().finish(flags);
    let mut dummy_environ =
        DummyEnvironment::new(isa.frontend_config(), ReturnMode::NormalReturns, false);
    translate_module(&data.wasm, &mut dummy_environ).unwrap();
});
