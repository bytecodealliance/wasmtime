use cranelift_codegen::settings;
use wasmtime_jit::{CompilationStrategy, Compiler, NullResolver};

#[test]
fn instantiate_empty_module() {
    // `(module)`
    let wasm = vec![0x0, 0x61, 0x73, 0x6d, 0x01, 0x0, 0x0, 0x0];
    let compilation_strategy = CompilationStrategy::Cranelift;

    let isa = {
        let flag_builder = settings::builder();
        let isa_builder =
            cranelift_native::builder().expect("host machine is not a supported target");
        isa_builder.finish(settings::Flags::new(flag_builder))
    };

    let mut compiler = Compiler::new(isa, compilation_strategy);
    let mut imports_resolver = NullResolver {};

    wasmtime_jit::instantiate(
        &mut compiler,
        &wasm,
        &mut imports_resolver,
        Default::default(),
        true,
    )
    .expect("failed to instantiate valid Wasm!");
}
