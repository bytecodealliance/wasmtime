use anyhow::{bail, Result};
use wasmtime::*;

fn serialize(engine: &Engine, wat: &'static str) -> Result<Vec<u8>> {
    let module = Module::new(&engine, wat)?;
    Ok(module.serialize()?)
}

unsafe fn deserialize_and_instantiate(store: &mut Store<()>, buffer: &[u8]) -> Result<Instance> {
    let module = Module::deserialize(store.engine(), buffer)?;
    Ok(Instance::new(store, &module, &[])?)
}

#[test]
fn test_version_mismatch() -> Result<()> {
    let engine = Engine::default();
    let mut buffer = serialize(&engine, "(module)")?;
    buffer[13 /* header length */ + 1 /* version length */] = 'x' as u8;

    match unsafe { Module::deserialize(&engine, &buffer) } {
        Ok(_) => bail!("expected deserialization to fail"),
        Err(e) => assert!(e
            .to_string()
            .starts_with("Module was compiled with incompatible Wasmtime version")),
    }

    // Test deserialize_check_wasmtime_version, which disables the logic which rejects the above.
    let mut config = Config::new();
    config.deserialize_check_wasmtime_version(false);
    let engine = Engine::new(&config).unwrap();
    unsafe { Module::deserialize(&engine, &buffer) }
        .expect("module with corrupt version should deserialize when check is disabled");

    Ok(())
}

#[test]
fn test_module_serialize_simple() -> Result<()> {
    let buffer = serialize(
        &Engine::default(),
        "(module (func (export \"run\") (result i32) i32.const 42))",
    )?;

    let mut store = Store::default();
    let instance = unsafe { deserialize_and_instantiate(&mut store, &buffer)? };
    let run = instance.get_typed_func::<(), i32, _>(&mut store, "run")?;
    let result = run.call(&mut store, ())?;

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
    let mut store = Store::new(&Engine::new(&config)?, ());
    match unsafe { deserialize_and_instantiate(&mut store, &buffer) } {
        Ok(_) => bail!("expected failure at deserialization"),
        Err(_) => (),
    }
    Ok(())
}
