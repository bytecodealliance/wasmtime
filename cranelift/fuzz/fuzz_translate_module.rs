#![no_main]

#[macro_use]
extern crate libfuzzer_sys;
extern crate binaryen;
extern crate cranelift_codegen;
extern crate cranelift_wasm;
#[macro_use]
extern crate target_lexicon;

use cranelift_codegen::{isa, settings};
use cranelift_wasm::{translate_module, DummyEnvironment, ReturnMode};
use std::str::FromStr;

fuzz_target!(|data: &[u8]| {
    let binaryen_module = binaryen::tools::translate_to_fuzz_mvp(data);

    let wasm = binaryen_module.write();

    let flags = settings::Flags::new(settings::builder());
    let triple = triple!("x86_64");
    let isa = isa::lookup(triple).unwrap().finish(flags);
    let mut dummy_environ = DummyEnvironment::new(isa.frontend_config(), ReturnMode::NormalReturns);
    translate_module(&wasm, &mut dummy_environ).unwrap();
});
