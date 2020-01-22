use more_asserts::assert_gt;
use std::path::PathBuf;
use wasmtime_environ::settings;
use wasmtime_environ::settings::Configurable;
use wasmtime_environ::CacheConfig;
use wasmtime_jit::{instantiate, native, CompilationStrategy, Compiler, NullResolver};

const PATH_MODULE_RS2WASM_ADD_FUNC: &str = r"tests/wat/rs2wasm-add-func.wat";

/// Simple test reading a wasm-file and translating to binary representation.
#[test]
fn test_environ_translate() {
    let path = PathBuf::from(PATH_MODULE_RS2WASM_ADD_FUNC);
    let data = wat::parse_file(path).expect("expecting valid wat-file");
    assert_gt!(data.len(), 0);

    let mut flag_builder = settings::builder();
    flag_builder.enable("enable_verifier").unwrap();

    let isa_builder = native::builder();
    let isa = isa_builder.finish(settings::Flags::new(flag_builder));

    let mut resolver = NullResolver {};
    let cache_config = CacheConfig::new_cache_disabled();
    let mut compiler = Compiler::new(isa, CompilationStrategy::Auto, cache_config);
    unsafe {
        let instance = instantiate(&mut compiler, &data, &mut resolver, false);
        assert!(instance.is_ok());
    }
}
