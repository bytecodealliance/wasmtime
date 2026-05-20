#![cfg(arc_try_new)]

use wasmtime::{Config, Engine, Linker, Memory, MemoryType, Module, Result, Store, Val};
use wasmtime_fuzzing::oom::OomTest;

#[test]
fn memory_new() -> Result<()> {
    let mut config = Config::new();
    config.enable_compiler(false);
    config.concurrency_support(false);
    let engine = Engine::new(&config)?;

    OomTest::new()
        // `IndexMap::reserve` will try to allocate double space, but if that
        // fails, will attempt to allocate the minimal space necessary.
        .allow_alloc_after_oom(true)
        .test(|| {
            let mut store = Store::try_new(&engine, ())?;
            let _memory = Memory::new(&mut store, MemoryType::new(1, None))?;
            Ok(())
        })
}

#[tokio::test]
async fn memory_new_async() -> Result<()> {
    let mut config = Config::new();
    config.enable_compiler(false);
    config.concurrency_support(false);
    let engine = Engine::new(&config)?;

    OomTest::new()
        .allow_alloc_after_oom(true)
        .test_async(|| async {
            let mut store = Store::try_new(&engine, ())?;
            let _memory = Memory::new_async(&mut store, MemoryType::new(1, None)).await?;
            Ok(())
        })
        .await
}

#[test]
fn memory_grow() -> Result<()> {
    let mut config = Config::new();
    config.enable_compiler(false);
    config.concurrency_support(false);
    let engine = Engine::new(&config)?;

    OomTest::new().allow_alloc_after_oom(true).test(|| {
        let mut store = Store::try_new(&engine, ())?;
        let memory = Memory::new(&mut store, MemoryType::new(1, None))?;
        let _old_size = memory.grow(&mut store, 1)?;
        Ok(())
    })
}

#[tokio::test]
async fn memory_grow_async() -> Result<()> {
    let mut config = Config::new();
    config.enable_compiler(false);
    config.concurrency_support(false);
    let engine = Engine::new(&config)?;

    OomTest::new()
        .allow_alloc_after_oom(true)
        .test_async(|| async {
            let mut store = Store::try_new(&engine, ())?;
            let memory = Memory::new_async(&mut store, MemoryType::new(1, None)).await?;
            let _old_size = memory.grow_async(&mut store, 1).await?;
            Ok(())
        })
        .await
}

#[test]
fn memory_ty() -> Result<()> {
    let mut config = Config::new();
    config.enable_compiler(false);
    config.concurrency_support(false);
    let engine = Engine::new(&config)?;

    OomTest::new().allow_alloc_after_oom(true).test(|| {
        let mut store = Store::try_new(&engine, ())?;
        let memory = Memory::new(&mut store, MemoryType::new(1, None))?;
        let _ty = memory.ty(&store);
        Ok(())
    })
}

#[test]
fn memory_size() -> Result<()> {
    let mut config = Config::new();
    config.enable_compiler(false);
    config.concurrency_support(false);
    let engine = Engine::new(&config)?;

    OomTest::new().allow_alloc_after_oom(true).test(|| {
        let mut store = Store::try_new(&engine, ())?;
        let memory = Memory::new(&mut store, MemoryType::new(1, None))?;
        assert_eq!(memory.size(&store), 1);
        Ok(())
    })
}

#[test]
fn memory_data_size() -> Result<()> {
    let mut config = Config::new();
    config.enable_compiler(false);
    config.concurrency_support(false);
    let engine = Engine::new(&config)?;

    OomTest::new().allow_alloc_after_oom(true).test(|| {
        let mut store = Store::try_new(&engine, ())?;
        let memory = Memory::new(&mut store, MemoryType::new(1, None))?;
        assert_eq!(memory.data_size(&store), 65536);
        Ok(())
    })
}

#[test]
fn memory_read_write() -> Result<()> {
    let mut config = Config::new();
    config.enable_compiler(false);
    config.concurrency_support(false);
    let engine = Engine::new(&config)?;

    OomTest::new().allow_alloc_after_oom(true).test(|| {
        let mut store = Store::try_new(&engine, ())?;
        let memory = Memory::new(&mut store, MemoryType::new(1, None))?;
        memory.write(&mut store, 0, &[1, 2, 3, 4])?;
        let mut buf = [0u8; 4];
        memory.read(&store, 0, &mut buf)?;
        assert_eq!(buf, [1, 2, 3, 4]);
        Ok(())
    })
}

#[test]
fn memory_data() -> Result<()> {
    let mut config = Config::new();
    config.enable_compiler(false);
    config.concurrency_support(false);
    let engine = Engine::new(&config)?;

    OomTest::new().allow_alloc_after_oom(true).test(|| {
        let mut store = Store::try_new(&engine, ())?;
        let memory = Memory::new(&mut store, MemoryType::new(1, None))?;
        let data = memory.data(&store);
        assert_eq!(data.len(), 65536);
        Ok(())
    })
}

#[test]
fn memory_data_mut() -> Result<()> {
    let mut config = Config::new();
    config.enable_compiler(false);
    config.concurrency_support(false);
    let engine = Engine::new(&config)?;

    OomTest::new().allow_alloc_after_oom(true).test(|| {
        let mut store = Store::try_new(&engine, ())?;
        let memory = Memory::new(&mut store, MemoryType::new(1, None))?;
        let data = memory.data_mut(&mut store);
        data[0] = 42;
        assert_eq!(data[0], 42);
        Ok(())
    })
}

#[test]
fn wasm_memory_grow_relocate() -> Result<()> {
    let module_bytes = {
        let mut config = Config::new();
        config.concurrency_support(false);
        config.memory_reservation(65536);
        config.memory_reservation_for_growth(0);
        let engine = Engine::new(&config)?;
        Module::new(
            &engine,
            r#"(module
                (memory (export "memory") 1)
                (func (export "grow") (param i32) (result i32)
                    (memory.grow (local.get 0))
                )
            )"#,
        )?
        .serialize()?
    };
    let mut config = Config::new();
    config.enable_compiler(false);
    config.concurrency_support(false);
    config.memory_reservation(65536);
    config.memory_reservation_for_growth(0);
    let engine = Engine::new(&config)?;
    let module = unsafe { Module::deserialize(&engine, &module_bytes)? };
    let linker = Linker::<()>::new(&engine);
    let instance_pre = linker.instantiate_pre(&module)?;

    OomTest::new()
        .allow_alloc_after_oom(true)
        .alloc_succeeds_after_oom(true)
        .allow_missed_oom_errors(true)
        .test(|| {
            let mut store = Store::try_new(&engine, ())?;
            let instance = instance_pre.instantiate(&mut store)?;
            let grow = instance.get_func(&mut store, "grow").unwrap();
            let mut results = [Val::I32(0)];
            grow.call(&mut store, &[Val::I32(1)], &mut results)?;
            Ok(())
        })
}
