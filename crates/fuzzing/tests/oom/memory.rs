#![cfg(arc_try_new)]

use wasmtime::{Config, Engine, Memory, MemoryType, Result, Store};
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
