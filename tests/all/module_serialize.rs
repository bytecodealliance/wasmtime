use anyhow::{bail, Result};
use wasmtime::*;

fn serialize(engine: &Engine, wat: &'static str) -> Result<Vec<u8>> {
    let module = Module::new(&engine, wat)?;
    Ok(module.serialize()?)
}

fn deserialize_and_instantiate(store: &Store, buffer: &[u8]) -> Result<Instance> {
    let module = Module::deserialize(store.engine(), buffer)?;
    Ok(Instance::new(&store, &module, &[])?)
}

#[test]
fn test_module_serialize_simple() -> Result<()> {
    let buffer = serialize(
        &Engine::default(),
        "(module (func (export \"run\") (result i32) i32.const 42))",
    )?;

    let store = Store::default();
    let instance = deserialize_and_instantiate(&store, &buffer)?;
    let run = instance.get_typed_func::<(), i32>("run")?;
    let result = run.call(())?;

    assert_eq!(42, result);
    Ok(())
}

#[test]
fn test_module_serialize_fail() -> Result<()> {
    let buffer = serialize(
        &Engine::default(),
        "(module (func (export \"run\") (result i32) i32.const 42))",
    )?;

    let mut config = Config::new();
    config.cranelift_opt_level(OptLevel::None);
    let store = Store::new(&Engine::new(&config)?);
    match deserialize_and_instantiate(&store, &buffer) {
        Ok(_) => bail!("expected failure at deserialization"),
        Err(_) => (),
    }
    Ok(())
}
