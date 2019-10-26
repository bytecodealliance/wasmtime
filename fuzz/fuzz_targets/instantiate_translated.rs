#![no_main]

#[macro_use]
extern crate libfuzzer_sys;
extern crate cranelift_codegen;
extern crate cranelift_native;
extern crate wasmparser;
extern crate wasmtime_environ;
extern crate wasmtime_jit;

use cranelift_codegen::settings;
use wasmtime_jit::{instantiate, Compiler, NullResolver};

fuzz_target!(|data: &[u8]| {
    let binaryen_module = binaryen::tools::translate_to_fuzz_mvp(data);
    let wasm = binaryen_module.write();
    let flag_builder = settings::builder();
    let isa_builder = cranelift_native::builder().unwrap_or_else(|_| {
        panic!("host machine is not a supported target");
    });
    let isa = isa_builder.finish(settings::Flags::new(flag_builder));
    let mut compiler = Compiler::new(isa);
    let mut imports_resolver = NullResolver {};
    let _instance = instantiate(
        &mut compiler,
        &wasm,
        &mut imports_resolver,
        Default::default(),
        true,
    )
    .unwrap();
});
