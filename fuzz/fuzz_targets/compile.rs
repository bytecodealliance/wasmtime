#![no_main]

#[macro_use]
extern crate libfuzzer_sys;
extern crate cranelift_codegen;
extern crate cranelift_wasm;
extern crate cranelift_native;
extern crate wasmtime_runtime;
extern crate wasmtime_execute;
extern crate wasmparser;

use cranelift_codegen::settings;
use cranelift_wasm::translate_module;
use wasmtime_runtime::{ModuleEnvironment, Module};
use wasmparser::{validate};

fuzz_target!(|data: &[u8]| {
    if !validate(data, None) {
        return;
    }
    let (flag_builder, isa_builder) = cranelift_native::builders().unwrap_or_else(|_| {
        panic!("host machine is not a supported target");
    });
    let isa = isa_builder.finish(settings::Flags::new(flag_builder));
    let mut module = Module::new();
    let mut runtime = ModuleEnvironment::new(&*isa, &mut module);
    let translation = match translate_module(&data, &mut runtime) {
        Ok(()) => (),
        Err(_) => return,
    };
    let _exec = match wasmtime_execute::compile_module(&*isa, &translation) {
        Ok(x) => x,
        Err(_) => return,
    };
});
