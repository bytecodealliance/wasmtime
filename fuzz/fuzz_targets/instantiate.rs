#![no_main]

extern crate libfuzzer_sys;

use cranelift_codegen::settings;
use libfuzzer_sys::fuzz_target;
use wasmparser::validate;
use wasmtime_jit::{instantiate, CompilationStrategy, Compiler, NullResolver};

fuzz_target!(|data: &[u8]| {
    if validate(data, None).is_err() {
        return;
    }
    let flag_builder = settings::builder();
    let isa_builder = cranelift_native::builder().unwrap_or_else(|_| {
        panic!("host machine is not a supported target");
    });
    let isa = isa_builder.finish(settings::Flags::new(flag_builder));
    let mut compiler = Compiler::new(isa, CompilationStrategy::Auto);
    let mut imports_resolver = NullResolver {};
    let _instance = instantiate(
        &mut compiler,
        data,
        &mut imports_resolver,
        Default::default(),
        true,
    )
    .unwrap();
});
