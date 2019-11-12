#![no_main]

extern crate libfuzzer_sys;

use cranelift_codegen::settings;
use libfuzzer_sys::fuzz_target;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use wasmparser::validate;
use wasmtime_jit::{CompilationStrategy, CompiledModule, Compiler, NullResolver};

fuzz_target!(|data: &[u8]| {
    if validate(data, None).is_err() {
        return;
    }
    let flag_builder = settings::builder();
    let isa_builder = cranelift_native::builder().unwrap_or_else(|_| {
        panic!("host machine is not a supported target");
    });
    let isa = isa_builder.finish(settings::Flags::new(flag_builder));
    let mut compiler = Compiler::new(isa, CompilationStrategy::Cranelift);
    let mut resolver = NullResolver {};
    let global_exports = Rc::new(RefCell::new(HashMap::new()));
    let _compiled =
        match CompiledModule::new(&mut compiler, data, &mut resolver, global_exports, false) {
            Ok(x) => x,
            Err(_) => return,
        };
});

#[cfg(feature = "lightbeam")]
fuzz_target!(|data: &[u8]| {
    if validate(data, None).is_err() {
        return;
    }
    let flag_builder = settings::builder();
    let isa_builder = cranelift_native::builder().unwrap_or_else(|_| {
        panic!("host machine is not a supported target");
    });
    let isa = isa_builder.finish(settings::Flags::new(flag_builder));
    let mut compiler = Compiler::new(isa, CompilationStrategy::Lightbeam);
    let mut resolver = NullResolver {};
    let global_exports = Rc::new(RefCell::new(HashMap::new()));
    let _compiled =
        match CompiledModule::new(&mut compiler, data, &mut resolver, global_exports, false) {
            Ok(x) => x,
            Err(_) => return,
        };
});
