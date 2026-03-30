#![cfg(arc_try_new)]

use wasmtime::{Config, Engine, Result, Store, StoreLimitsBuilder};
use wasmtime_fuzzing::oom::OomTest;

#[test]
fn store_try_new() -> Result<()> {
    let mut config = Config::new();
    config.enable_compiler(false);
    config.concurrency_support(false);
    let engine = Engine::new(&config)?;
    OomTest::new().test(|| {
        let _ = Store::try_new(&engine, ())?;
        Ok(())
    })
}

#[test]
fn store_data() -> Result<()> {
    let mut config = Config::new();
    config.enable_compiler(false);
    config.concurrency_support(false);
    let engine = Engine::new(&config)?;
    OomTest::new().test(|| {
        let store = Store::try_new(&engine, 42u32)?;
        assert_eq!(*store.data(), 42);
        Ok(())
    })
}

#[test]
fn store_data_mut() -> Result<()> {
    let mut config = Config::new();
    config.enable_compiler(false);
    config.concurrency_support(false);
    let engine = Engine::new(&config)?;
    OomTest::new().test(|| {
        let mut store = Store::try_new(&engine, 42u32)?;
        *store.data_mut() = 99;
        assert_eq!(*store.data(), 99);
        Ok(())
    })
}

#[test]
fn store_engine() -> Result<()> {
    let mut config = Config::new();
    config.enable_compiler(false);
    config.concurrency_support(false);
    let engine = Engine::new(&config)?;
    OomTest::new().test(|| {
        let store = Store::try_new(&engine, ())?;
        let _engine = store.engine();
        Ok(())
    })
}

#[test]
fn store_limiter() -> Result<()> {
    let mut config = Config::new();
    config.enable_compiler(false);
    config.concurrency_support(false);
    let engine = Engine::new(&config)?;
    OomTest::new().test(|| {
        let mut store = Store::try_new(&engine, StoreLimitsBuilder::new().build())?;
        store.limiter(|limits| limits);
        Ok(())
    })
}

#[test]
fn store_fuel() -> Result<()> {
    let mut config = Config::new();
    config.enable_compiler(false);
    config.concurrency_support(false);
    config.consume_fuel(true);
    let engine = Engine::new(&config)?;
    OomTest::new().test(|| {
        let mut store = Store::try_new(&engine, ())?;
        store.set_fuel(100)?;
        let fuel = store.get_fuel()?;
        assert_eq!(fuel, 100);
        Ok(())
    })
}

#[test]
fn store_epoch_deadline() -> Result<()> {
    let mut config = Config::new();
    config.enable_compiler(false);
    config.concurrency_support(false);
    config.epoch_interruption(true);
    let engine = Engine::new(&config)?;
    OomTest::new().test(|| {
        let mut store = Store::try_new(&engine, ())?;
        store.set_epoch_deadline(10);
        Ok(())
    })
}
