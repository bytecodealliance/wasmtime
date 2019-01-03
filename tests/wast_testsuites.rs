extern crate cranelift_codegen;
extern crate cranelift_native;
extern crate wasmtime_jit;
extern crate wasmtime_wast;

use cranelift_codegen::isa;
use cranelift_codegen::settings;
use cranelift_codegen::settings::Configurable;
use std::path::Path;
use wasmtime_jit::Compiler;
use wasmtime_wast::WastContext;

include!(concat!(env!("OUT_DIR"), "/wast_testsuite_tests.rs"));

#[cfg(test)]
fn native_isa() -> Box<isa::TargetIsa> {
    let mut flag_builder = settings::builder();
    flag_builder.enable("enable_verifier").unwrap();

    let isa_builder = cranelift_native::builder().unwrap_or_else(|_| {
        panic!("host machine is not a supported target");
    });

    isa_builder.finish(settings::Flags::new(flag_builder))
}
