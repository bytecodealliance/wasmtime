#![cfg(not(miri))]

use wasm_encoder::Instruction as I;
use wasmtime::*;

#[test]
fn code_too_large_without_panic() -> Result<()> {
    const N: usize = 80000;

    // Build a module with a function whose body will allocate too many
    // temporaries for our current (Cranelift-based) compiler backend to
    // handle. This test ensures that we propagate the failure upward
    // and return it programmatically, rather than panic'ing. If we ever
    // improve our compiler backend to actually handle such a large
    // function body, we'll need to increase the limits here too!
    let mut module = wasm_encoder::Module::default();

    let mut types = wasm_encoder::TypeSection::new();
    types.ty().function([], [wasm_encoder::ValType::I32]);
    module.section(&types);

    let mut funcs = wasm_encoder::FunctionSection::new();
    funcs.function(0);
    module.section(&funcs);

    let mut tables = wasm_encoder::TableSection::new();
    tables.table(wasm_encoder::TableType {
        element_type: wasm_encoder::RefType::FUNCREF,
        table64: false,
        minimum: 1,
        maximum: Some(1),
        shared: false,
    });
    module.section(&tables);

    let mut exports = wasm_encoder::ExportSection::new();
    exports.export("", wasm_encoder::ExportKind::Func, 0);
    module.section(&exports);

    let mut func = wasm_encoder::Function::new([]);
    func.instruction(&I::I32Const(0));
    for _ in 0..N {
        func.instruction(&I::TableGet(0));
        func.instruction(&I::RefIsNull);
    }
    func.instruction(&I::End);
    let mut code = wasm_encoder::CodeSection::new();
    code.function(&func);
    module.section(&code);

    let mut config = Config::new();
    config.cranelift_opt_level(OptLevel::None);
    let engine = Engine::new(&config)?;

    let store = Store::new(&engine, ());
    let result = Module::new(store.engine(), &module.finish());
    match result {
        Err(e) => assert!(
            e.to_string()
                .starts_with("Compilation error: Code for function is too large")
        ),
        Ok(_) => panic!("Please adjust limits to make the module too large to compile!"),
    }
    Ok(())
}
