use cranelift_codegen::settings;
use cranelift_codegen::settings::Configurable;
use more_asserts::assert_gt;
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;
use wasmtime_jit::{instantiate, CompilationStrategy, Compiler, NullResolver};

const PATH_MODULE_RS2WASM_ADD_FUNC: &str = r"tests/wat/rs2wasm-add-func.wat";

/// Simple test reading a wasm-file and translating to binary representation.
#[test]
fn test_environ_translate() {
    let path = PathBuf::from(PATH_MODULE_RS2WASM_ADD_FUNC);
    let data = wat::parse_file(path).expect("expecting valid wat-file");
    assert_gt!(data.len(), 0);

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
