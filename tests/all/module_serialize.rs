use anyhow::bail;
use std::fs::{self, OpenOptions};
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
    let buffer = serialize(&engine, "(module)")?;

    let mut config = Config::new();
    config
        .module_version(ModuleVersionStrategy::Custom("custom!".to_owned()))
        .unwrap();
    let custom_version_engine = Engine::new(&config).unwrap();
    match unsafe { Module::deserialize(&custom_version_engine, &buffer) } {
        Ok(_) => bail!("expected deserialization to fail"),
        Err(e) => assert!(
            e.to_string()
                .starts_with("Module was compiled with incompatible version")
        ),
    }

    let mut config = Config::new();
    config.module_version(ModuleVersionStrategy::None).unwrap();
    let none_version_engine = Engine::new(&config).unwrap();
    unsafe { Module::deserialize(&none_version_engine, &buffer) }
        .expect("accepts the wasmtime versioned module");

    let buffer = serialize(&custom_version_engine, "(module)")?;
    unsafe { Module::deserialize(&none_version_engine, &buffer) }
        .expect("accepts the custom versioned module");

    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn test_module_serialize_simple() -> Result<()> {
    let buffer = serialize(
        &Engine::default(),
        "(module (func (export \"run\") (result i32) i32.const 42))",
    )?;

    let mut store = Store::default();
    let instance = unsafe { deserialize_and_instantiate(&mut store, &buffer)? };
    let run = instance.get_typed_func::<(), i32>(&mut store, "run")?;
    let result = run.call(&mut store, ())?;

    assert_eq!(42, result);
    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn test_module_serialize_fail() -> Result<()> {
    let buffer = serialize(
        &Engine::default(),
        "(module (func (export \"run\") (result i32) i32.const 42))",
    )?;

    let mut config = Config::new();
    config.memory_reservation(0);
    let mut store = Store::new(&Engine::new(&config)?, ());
    match unsafe { deserialize_and_instantiate(&mut store, &buffer) } {
        Ok(_) => bail!("expected failure at deserialization"),
        Err(_) => (),
    }
    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
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
        let func = instance.get_typed_func::<(), i32>(&mut store, "run")?;
        assert_eq!(func.call(&mut store, ())?, 42);

        // Try an already opened file as well.
        let mut open_options = OpenOptions::new();
        open_options.read(true);
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::prelude::*;
            use windows_sys::Win32::Storage::FileSystem::*;
            open_options.access_mode(FILE_GENERIC_READ | FILE_GENERIC_EXECUTE);
        }

        let file = open_options.open(&path)?;
        let module = unsafe { Module::deserialize_open_file(store.engine(), file)? };
        let instance = Instance::new(&mut store, &module, &[])?;
        let func = instance.get_typed_func::<(), i32>(&mut store, "run")?;
        assert_eq!(func.call(&mut store, ())?, 42);

        Ok(())
    }
}

#[test]
#[cfg_attr(miri, ignore)]
fn deserialize_from_serialized() -> Result<()> {
    let engine = Engine::default();
    let buffer1 = serialize(
        &engine,
        "(module (func (export \"run\") (result i32) i32.const 42))",
    )?;
    let buffer2 = unsafe { Module::deserialize(&engine, &buffer1)?.serialize()? };
    assert!(buffer1 == buffer2);
    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn detect_precompiled() -> Result<()> {
    let engine = Engine::default();
    let buffer = serialize(
        &engine,
        "(module (func (export \"run\") (result i32) i32.const 42))",
    )?;
    assert_eq!(engine.detect_precompiled(&[]), None);
    assert_eq!(engine.detect_precompiled(&buffer[..5]), None);
    assert_eq!(
        engine.detect_precompiled(&buffer),
        Some(Precompiled::Module)
    );
    Ok(())
}
