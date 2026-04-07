#![cfg(arc_try_new)]

use wasmtime::{Config, Engine, Ref, RefType, Result, Store, Table, TableType};
use wasmtime_fuzzing::oom::OomTest;

#[test]
fn table_new() -> Result<()> {
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
            let ty = TableType::new(RefType::FUNCREF, 1, None);
            let _table = wasmtime::Table::new(&mut store, ty, Ref::Func(None))?;
            Ok(())
        })
}

#[tokio::test]
async fn table_new_async() -> Result<()> {
    let mut config = Config::new();
    config.enable_compiler(false);
    config.concurrency_support(false);
    let engine = Engine::new(&config)?;

    OomTest::new()
        .allow_alloc_after_oom(true)
        .test_async(|| async {
            let mut store = Store::try_new(&engine, ())?;
            let ty = TableType::new(RefType::FUNCREF, 1, None);
            let _table = Table::new_async(&mut store, ty, Ref::Func(None)).await?;
            Ok(())
        })
        .await
}

#[test]
fn table_grow() -> Result<()> {
    let mut config = Config::new();
    config.enable_compiler(false);
    config.concurrency_support(false);
    let engine = Engine::new(&config)?;

    OomTest::new().allow_alloc_after_oom(true).test(|| {
        let mut store = Store::try_new(&engine, ())?;
        let ty = TableType::new(RefType::FUNCREF, 1, None);
        let table = Table::new(&mut store, ty, Ref::Func(None))?;
        let _old_size = table.grow(&mut store, 4, Ref::Func(None))?;
        Ok(())
    })
}

#[test]
fn table_set() -> Result<()> {
    let mut config = Config::new();
    config.enable_compiler(false);
    config.concurrency_support(false);
    let engine = Engine::new(&config)?;

    OomTest::new().allow_alloc_after_oom(true).test(|| {
        let mut store = Store::try_new(&engine, ())?;
        let ty = TableType::new(RefType::FUNCREF, 1, None);
        let table = Table::new(&mut store, ty, Ref::Func(None))?;
        table.set(&mut store, 0, Ref::Func(None))?;
        Ok(())
    })
}

#[test]
fn table_copy() -> Result<()> {
    let mut config = Config::new();
    config.enable_compiler(false);
    config.concurrency_support(false);
    let engine = Engine::new(&config)?;

    OomTest::new().allow_alloc_after_oom(true).test(|| {
        let mut store = Store::try_new(&engine, ())?;
        let ty = TableType::new(RefType::FUNCREF, 4, None);
        let table = Table::new(&mut store, ty, Ref::Func(None))?;
        Table::copy(&mut store, &table, 0, &table, 2, 2)?;
        Ok(())
    })
}

#[test]
fn table_fill() -> Result<()> {
    let mut config = Config::new();
    config.enable_compiler(false);
    config.concurrency_support(false);
    let engine = Engine::new(&config)?;

    OomTest::new().allow_alloc_after_oom(true).test(|| {
        let mut store = Store::try_new(&engine, ())?;
        let ty = TableType::new(RefType::FUNCREF, 4, None);
        let table = Table::new(&mut store, ty, Ref::Func(None))?;
        table.fill(&mut store, 0, Ref::Func(None), 4)?;
        Ok(())
    })
}

#[test]
fn table_get() -> Result<()> {
    let mut config = Config::new();
    config.enable_compiler(false);
    config.concurrency_support(false);
    let engine = Engine::new(&config)?;

    OomTest::new().allow_alloc_after_oom(true).test(|| {
        let mut store = Store::try_new(&engine, ())?;
        let ty = TableType::new(RefType::FUNCREF, 1, None);
        let table = Table::new(&mut store, ty, Ref::Func(None))?;
        let val = table.get(&mut store, 0);
        assert!(val.is_some());
        Ok(())
    })
}

#[test]
fn table_ty() -> Result<()> {
    let mut config = Config::new();
    config.enable_compiler(false);
    config.concurrency_support(false);
    let engine = Engine::new(&config)?;

    OomTest::new().allow_alloc_after_oom(true).test(|| {
        let mut store = Store::try_new(&engine, ())?;
        let ty = TableType::new(RefType::FUNCREF, 1, None);
        let table = Table::new(&mut store, ty, Ref::Func(None))?;
        let _ty = table.ty(&store);
        Ok(())
    })
}

#[test]
fn table_size() -> Result<()> {
    let mut config = Config::new();
    config.enable_compiler(false);
    config.concurrency_support(false);
    let engine = Engine::new(&config)?;

    OomTest::new().allow_alloc_after_oom(true).test(|| {
        let mut store = Store::try_new(&engine, ())?;
        let ty = TableType::new(RefType::FUNCREF, 2, None);
        let table = Table::new(&mut store, ty, Ref::Func(None))?;
        assert_eq!(table.size(&store), 2);
        Ok(())
    })
}
