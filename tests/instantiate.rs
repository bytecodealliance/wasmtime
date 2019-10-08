extern crate alloc;

use alloc::rc::Rc;
use core::cell::RefCell;
use cranelift_codegen::settings;
use cranelift_codegen::settings::Configurable;
use std::collections::HashMap;
use std::fs::File;
use std::io;
use std::io::Read;
use std::path::PathBuf;
use wabt;
use wasmtime_jit::{instantiate, CompilationStrategy, Compiler, NullResolver};

#[cfg(test)]
const PATH_MODULE_RS2WASM_ADD_FUNC: &str = r"filetests/rs2wasm-add-func.wat";

#[cfg(test)]
fn read_to_end(path: PathBuf) -> Result<Vec<u8>, io::Error> {
    let mut buf: Vec<u8> = Vec::new();
    let mut file = File::open(path)?;
    file.read_to_end(&mut buf)?;
    Ok(buf)
}

/// Simple test reading a wasm-file and translating to binary representation.
#[test]
fn test_environ_translate() {
    let path = PathBuf::from(PATH_MODULE_RS2WASM_ADD_FUNC);
    let wat_data = read_to_end(path).unwrap();
    assert!(wat_data.len() > 0);

    let data = wabt::wat2wasm(wat_data).expect("expecting valid wat-file");
    assert!(data.len() > 0);

    let mut flag_builder = settings::builder();
    flag_builder.enable("enable_verifier").unwrap();

    let isa_builder = cranelift_native::builder().unwrap_or_else(|_| {
        panic!("host machine is not a supported target");
    });
    let isa = isa_builder.finish(settings::Flags::new(flag_builder));

    let mut resolver = NullResolver {};
    let mut compiler = Compiler::new(isa, CompilationStrategy::Auto);
    let global_exports = Rc::new(RefCell::new(HashMap::new()));
    let instance = instantiate(&mut compiler, &data, &mut resolver, global_exports, false);
    assert!(instance.is_ok());
}
