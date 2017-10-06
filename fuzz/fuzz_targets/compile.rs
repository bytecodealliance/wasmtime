#![no_main]

#[macro_use]
extern crate libfuzzer_sys;
extern crate cretonne;
extern crate cton_wasm;
extern crate cton_native;
extern crate wasmstandalone_runtime;
extern crate wasmstandalone_execute;

use cretonne::settings;
use cton_wasm::translate_module;

fuzz_target!(|data: &[u8]| {
    let (flag_builder, isa_builder) = cton_native::builders().unwrap_or_else(|_| {
        panic!("host machine is not a supported target");
    });
    let isa = isa_builder.finish(settings::Flags::new(&flag_builder));
    let mut runtime = wasmstandalone_runtime::Runtime::with_flags(isa.flags().clone());
    let translation = match translate_module(&data, &mut runtime) {
        Ok(x) => x,
        Err(_) => return,
    };
    let _exec = match wasmstandalone_execute::compile_module(&translation, &*isa, &runtime) {
        Ok(x) => x,
        Err(_) => return,
    };
});
