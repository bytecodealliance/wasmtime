#![cfg(arc_try_new)]

use wasmtime::{Config, Engine, MemoryTypeBuilder, Result, SharedMemory};
use wasmtime_fuzzing::oom::OomTest;

fn shared_memory_engine() -> Result<Engine> {
    let mut config = Config::new();
    config.enable_compiler(false);
    config.shared_memory(true);
    Engine::new(&config)
}

#[test]
fn shared_memory_new() -> Result<()> {
    let engine = shared_memory_engine()?;

    OomTest::new().test(|| {
        let ty = MemoryTypeBuilder::new()
            .min(1)
            .max(Some(2))
            .shared(true)
            .build()?;
        let _mem = SharedMemory::new(&engine, ty)?;
        Ok(())
    })
}

#[test]
fn shared_memory_ty() -> Result<()> {
    let engine = shared_memory_engine()?;

    OomTest::new().test(|| {
        let ty = MemoryTypeBuilder::new()
            .min(1)
            .max(Some(2))
            .shared(true)
            .build()?;
        let mem = SharedMemory::new(&engine, ty)?;
        let _ty = mem.ty();
        Ok(())
    })
}

#[test]
fn shared_memory_size() -> Result<()> {
    let engine = shared_memory_engine()?;

    OomTest::new().test(|| {
        let ty = MemoryTypeBuilder::new()
            .min(1)
            .max(Some(2))
            .shared(true)
            .build()?;
        let mem = SharedMemory::new(&engine, ty)?;
        assert_eq!(mem.size(), 1);
        Ok(())
    })
}

#[test]
fn shared_memory_grow() -> Result<()> {
    let engine = shared_memory_engine()?;

    OomTest::new().test(|| {
        let ty = MemoryTypeBuilder::new()
            .min(1)
            .max(Some(4))
            .shared(true)
            .build()?;
        let mem = SharedMemory::new(&engine, ty)?;
        let _old = mem.grow(1)?;
        Ok(())
    })
}

#[test]
fn shared_memory_data_size() -> Result<()> {
    let engine = shared_memory_engine()?;

    OomTest::new().test(|| {
        let ty = MemoryTypeBuilder::new()
            .min(1)
            .max(Some(2))
            .shared(true)
            .build()?;
        let mem = SharedMemory::new(&engine, ty)?;
        assert_eq!(mem.data_size(), 65536);
        Ok(())
    })
}
