#![no_main]
#[macro_use]
extern crate libfuzzer_sys;
extern crate binaryen;
extern crate cretonne_wasm;
use cretonne_wasm::{translate_module, DummyEnvironment};

fuzz_target!(|data: &[u8]| {
    let binaryen_module = binaryen::tools::translate_to_fuzz_mvp(data);

    let wasm = binaryen_module.write();

    let mut dummy_environ = DummyEnvironment::default();
    translate_module(&wasm, &mut dummy_environ).unwrap();
});
