#![no_main]

#[macro_use]
extern crate libfuzzer_sys;
extern crate cranelift_codegen;
extern crate cranelift_native;
extern crate wasmtime_environ;
extern crate wasmtime_execute;
extern crate wasmparser;

use cranelift_codegen::settings;
use wasmtime_environ::{ModuleEnvironment, Module};
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
    let environment = ModuleEnvironment::new(&*isa, &mut module);
    let translation = match environment.translate(&data) {
        Ok(translation) => translation,
        Err(_) => return,
    };
    let _exec = match wasmtime_execute::compile_and_link_module(&*isa, &translation) {
        Ok(x) => x,
        Err(_) => return,
    };
});
