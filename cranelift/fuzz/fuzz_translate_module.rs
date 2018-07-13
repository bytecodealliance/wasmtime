#![no_main]
#[macro_use]
extern crate libfuzzer_sys;
extern crate binaryen;
extern crate cranelift_wasm;
#[macro_use]
extern crate target_lexicon;
use cranelift_wasm::{translate_module, DummyEnvironment};
use std::str::FromStr;

fuzz_target!(|data: &[u8]| {
    let binaryen_module = binaryen::tools::translate_to_fuzz_mvp(data);

    let wasm = binaryen_module.write();

    let mut dummy_environ = DummyEnvironment::with_triple(triple!("x86_64"));
    translate_module(&wasm, &mut dummy_environ).unwrap();
});
