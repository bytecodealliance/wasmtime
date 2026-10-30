use wasmtime::*;

pub fn engine() -> Option<Engine> {
    let mut config = Config::new();
    config.wasm_threads(true);
    config.shared_memory(true);
    match Engine::new(&config) {
        Ok(engine) => {
            assert!(cfg!(target_pointer_width = "64"));
            Some(engine)
        }
        Err(e) => {
            assert!(cfg!(target_pointer_width = "32"), "unexpected error {e:?}");
            None
        }
    }
}

#[test]
#[cfg_attr(miri, ignore)]
fn shared_memory_failed_creation() -> Result<()> {
    let mut config = Config::new();
    config.wasm_threads(true);
    config.shared_memory(false);
    let Ok(engine) = Engine::new(&config) else {
        return Ok(());
    };
    assert!(SharedMemory::new(&engine, MemoryType::shared(1, 1)?).is_err());
    let wat = r#"(module (memory 1 1 shared))"#;
    let module = Module::new(&engine, wat)?;
    let mut store = Store::new(&engine, ());
    assert!(Instance::new(&mut store, &module, &[]).is_err());
    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn test_export_shared_memory() -> Result<()> {
    let wat = r#"(module (memory (export "memory") 1 5 shared))"#;
    let Some(engine) = engine() else {
        return Ok(());
    };
    let module = Module::new(&engine, wat)?;
    let mut store = Store::new(&engine, ());
    let instance = Instance::new(&mut store, &module, &[])?;
    let shared_memory = instance.get_shared_memory(&mut store, "memory").unwrap();

    assert_eq!(shared_memory.size(), 1);
    assert!(shared_memory.ty().is_shared());
    assert_eq!(shared_memory.ty().maximum(), Some(5));

    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn test_probe_shared_memory_size() -> Result<()> {
    let wat = r#"(module
        (memory (export "memory") 1 2 shared)
        (func (export "size") (result i32) (memory.size))
    )"#;
    let Some(engine) = engine() else {
        return Ok(());
    };
    let module = Module::new(&engine, wat)?;
    let mut store = Store::new(&engine, ());
    let instance = Instance::new(&mut store, &module, &[])?;
    let size_fn = instance.get_typed_func::<(), i32>(&mut store, "size")?;
    let shared_memory = instance.get_shared_memory(&mut store, "memory").unwrap();

    assert_eq!(size_fn.call(&mut store, ())?, 1);
    assert_eq!(shared_memory.size(), 1);

    shared_memory.grow(1)?;

    assert_eq!(shared_memory.size(), 2);
    assert_eq!(size_fn.call(&mut store, ())?, 2);

    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn test_multi_memory() -> Result<()> {
    let wat = r#"(module
        (import "env" "imported" (memory $imported 5 10 shared))
        (memory (export "owned") 10 20)
        (memory (export "shared") 1 2 shared)
        (export "imported" (memory $imported))
    )"#;
    let Some(engine) = engine() else {
        return Ok(());
    };
    let module = Module::new(&engine, wat)?;
    let mut store = Store::new(&engine, ());
    let incoming_shared_memory = SharedMemory::new(&engine, MemoryType::shared(5, 10)?)?;
    let instance = Instance::new(&mut store, &module, &[incoming_shared_memory.into()])?;
    let owned_memory = instance.get_memory(&mut store, "owned").unwrap();
    let shared_memory = instance.get_shared_memory(&mut store, "shared").unwrap();
    let imported_memory = instance.get_shared_memory(&mut store, "imported").unwrap();

    assert_eq!(owned_memory.size(&store), 10);
    assert_eq!(owned_memory.ty(&store).minimum(), 10);
    assert_eq!(owned_memory.ty(&store).maximum(), Some(20));
    assert_eq!(owned_memory.ty(&store).is_shared(), false);
    assert_eq!(shared_memory.size(), 1);
    assert_eq!(shared_memory.ty().minimum(), 1);
    assert_eq!(shared_memory.ty().maximum(), Some(2));
    assert_eq!(shared_memory.ty().is_shared(), true);
    assert_eq!(imported_memory.size(), 5);
    assert_eq!(imported_memory.ty().minimum(), 5);
    assert_eq!(imported_memory.ty().maximum(), Some(10));
    assert_eq!(imported_memory.ty().is_shared(), true);

    Ok(())
}

#[test]
fn create_shared_memory_through_memory() -> Result<()> {
    let engine = Engine::default();
    let mut store = Store::new(&engine, ());
    assert!(Memory::new(&mut store, MemoryType::shared(1, 1)?).is_err());
    Ok(())
}
