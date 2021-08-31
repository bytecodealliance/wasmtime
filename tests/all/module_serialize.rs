use anyhow::{bail, Result};
use std::fs;
use wasmtime::*;

fn serialize(engine: &Engine, wat: &str) -> Result<Vec<u8>> {
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
    const HEADER: &[u8] = b"\0wasmtime-aot";
    let pos = memchr::memmem::rfind_iter(&buffer, HEADER).next().unwrap();
    buffer[pos + HEADER.len() + 1 /* version length */] = 'x' as u8;

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

#[test]
fn test_deserialize_from_file() -> Result<()> {
    serialize_and_call("(module (func (export \"run\") (result i32) i32.const 42))")?;
    serialize_and_call(
        "(module
            (func (export \"run\") (result i32)
                call $answer)

            (func $answer (result i32)
                i32.const 42))
        ",
    )?;
    return Ok(());

    fn serialize_and_call(wat: &str) -> Result<()> {
        let mut store = Store::<()>::default();
        let td = tempfile::TempDir::new()?;
        let buffer = serialize(store.engine(), wat)?;

        let path = td.path().join("module.bin");
        fs::write(&path, &buffer)?;
        let module = unsafe { Module::deserialize_file(store.engine(), &path)? };
        let instance = Instance::new(&mut store, &module, &[])?;
        let func = instance.get_typed_func::<(), i32, _>(&mut store, "run")?;
        assert_eq!(func.call(&mut store, ())?, 42);
        Ok(())
    }
}
