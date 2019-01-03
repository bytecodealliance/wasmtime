#![no_main]

#[macro_use]
extern crate libfuzzer_sys;
extern crate cranelift_codegen;
extern crate cranelift_native;
extern crate wasmparser;
extern crate wasmtime_environ;
extern crate wasmtime_jit;

use cranelift_codegen::settings;
use wasmparser::validate;
use wasmtime_environ::{Module, ModuleEnvironment};

fuzz_target!(|data: &[u8]| {
    if !validate(data, None) {
        return;
    }
    let flag_builder = settings::builder();
    let isa_builder = cranelift_native::builder().unwrap_or_else(|_| {
        panic!("host machine is not a supported target");
    });
    let isa = isa_builder.finish(settings::Flags::new(flag_builder));
    let mut module = Module::new();
    let environment = ModuleEnvironment::new(&*isa, &mut module);
    let translation = match environment.translate(&data) {
        Ok(translation) => translation,
        Err(_) => return,
    };
    let imports_resolver = |_env: &str, _function: &str| None;
    let _exec = match wasmtime_jit::compile_and_link_module(&*isa, &translation, imports_resolver) {
        Ok(x) => x,
        Err(_) => return,
    };
});
