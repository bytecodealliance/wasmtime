use anyhow::Result;
use wasmtime::*;

fn engine() -> Engine {
    let mut config = Config::new();
    config.wasm_module_linking(true);
    Engine::new(&config)
}

#[test]
fn compile() -> Result<()> {
    let engine = engine();
    Module::new(&engine, "(module (module))")?;
    Module::new(&engine, "(module (module) (module))")?;
    Module::new(&engine, "(module (module (module)))")?;
    Module::new(
        &engine,
        "
            (module
                (func)
                (module (func))
                (module (func))
            )
        ",
    )?;
    let m = Module::new(
        &engine,
        "
            (module
                (global i32 (i32.const 0))
                (func)
                (module (memory 1) (func))
                (module (memory 2) (func))
                (module (table 2 funcref) (func))
                (module (global i64 (i64.const 0)) (func))
            )
        ",
    )?;
    assert_eq!(m.imports().len(), 0);
    assert_eq!(m.exports().len(), 0);
    let bytes = m.serialize()?;
    Module::deserialize(&engine, &bytes)?;
    assert_eq!(m.imports().len(), 0);
    assert_eq!(m.exports().len(), 0);
    Ok(())
}

#[test]
fn types() -> Result<()> {
    let engine = engine();
    Module::new(&engine, "(module (type (module)))")?;
    Module::new(&engine, "(module (type (instance)))")?;
    Ok(())
}
