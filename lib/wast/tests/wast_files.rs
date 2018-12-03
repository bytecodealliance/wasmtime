extern crate cranelift_codegen;
extern crate cranelift_native;
extern crate wasmtime_wast;

use cranelift_codegen::isa;
use cranelift_codegen::settings;
use cranelift_codegen::settings::Configurable;
use std::path::Path;
use wasmtime_wast::wast_file;

include!(concat!(env!("OUT_DIR"), "/run_wast_files.rs"));

#[cfg(test)]
fn native_isa() -> Box<isa::TargetIsa> {
    let mut flag_builder = settings::builder();
    flag_builder.enable("enable_verifier").unwrap();

    let isa_builder = cranelift_native::builder().unwrap_or_else(|_| {
        panic!("host machine is not a supported target");
    });
    isa_builder.finish(settings::Flags::new(flag_builder))
}
