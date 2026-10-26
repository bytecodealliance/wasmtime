#![cfg(arc_try_new)]

use wasmtime::{
    Config, Engine, Extern, Func, FuncType, Global, GlobalType, Instance, Memory, MemoryType,
    Module, Mutability, Ref, RefType, Result, Store, StoreLimitsBuilder, Table, TableType, Tag,
    TagType, Val, ValType,
};
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

/// Check that an OOM partway through creating a host object leaves the
/// store's arenas consistent: the instance arena and its resource counts,
/// the `FuncRefs` arena, and `host_globals`.
///
/// The counters are bumped before allocating, so an OOM can leave one high.
#[tokio::test]
async fn store_arenas_reuse_store() -> Result<()> {
    let mut config = Config::new();
    config.concurrency_support(false);
    config.wasm_exceptions(true);
    let engine = Engine::new(&config)?;

    // Importing one of each kind reads every arena back.
    let module = Module::new(
        &engine,
        r#"
            (module
                (import "" "f" (func $f (result i32)))
                (import "" "g" (global $g (mut i32)))
                (import "" "m" (memory 1))
                (import "" "t" (table 1 funcref))
                (func (export "run") (result i32)
                    (i32.add (call $f) (global.get $g))
                )
            )
        "#,
    )?;

    /// One of every host object that pushes into a store arena.
    fn make_host_objects(store: &mut Store<()>) -> Result<[Extern; 4]> {
        let func = Func::try_wrap(&mut *store, || 7i32)?;
        let global = Global::new(
            &mut *store,
            GlobalType::new(ValType::I32, Mutability::Var),
            Val::I32(35),
        )?;
        let memory = Memory::new(&mut *store, MemoryType::new(1, None)?)?;
        let table = Table::new(
            &mut *store,
            TableType::new(RefType::FUNCREF, 1, None)?,
            Ref::Func(None),
        )?;
        Ok([func.into(), global.into(), memory.into(), table.into()])
    }

    OomTest::new()
        // `IndexMap::reserve` will try to allocate double space, but if that
        // fails, will attempt to allocate the minimal space necessary.
        .allow_alloc_after_oom(true)
        .test_with_reuse_async(
            async || Store::try_new(&engine, ()),
            async |store: &mut Store<()>| {
                // Every API that pushes into a store arena.
                let _ = Func::try_new(
                    &mut *store,
                    FuncType::try_new(&engine, [ValType::I32], [ValType::I32])?,
                    |_caller, params, results| {
                        results[0] = params[0].clone();
                        Ok(())
                    },
                )?;
                let _ = make_host_objects(store)?;
                let _ = Memory::new_async(&mut *store, MemoryType::new(1, None)?).await?;
                let _ = Table::new_async(
                    &mut *store,
                    TableType::new(RefType::FUNCREF, 1, None)?,
                    Ref::Func(None),
                )
                .await?;
                let _ = Tag::new(
                    &mut *store,
                    &TagType::new(FuncType::try_new(&engine, [ValType::I32], [])?),
                )?;
                Ok(())
            },
            async |mut store: Store<()>| {
                let imports = make_host_objects(&mut store)?;

                let global = imports[1].clone().into_global().unwrap();
                assert_eq!(global.get(&mut store).unwrap_i32(), 35);
                global.set(&mut store, Val::I32(99))?;
                assert_eq!(global.get(&mut store).unwrap_i32(), 99);
                global.set(&mut store, Val::I32(35))?;
                assert!(global.ty(&store).content().is_i32());

                let memory = imports[2].clone().into_memory().unwrap();
                assert_eq!(memory.size(&store), 1);
                memory.write(&mut store, 0, &[1, 2, 3, 4])?;
                let mut buf = [0u8; 4];
                memory.read(&store, 0, &mut buf)?;
                assert_eq!(buf, [1, 2, 3, 4]);
                assert_eq!(memory.grow(&mut store, 1)?, 1);

                let table = imports[3].clone().into_table().unwrap();
                assert_eq!(table.size(&store), 1);
                assert!(table.get(&mut store, 0).is_some());
                assert_eq!(table.grow(&mut store, 1, Ref::Func(None))?, 1);

                let tag_ty = TagType::new(FuncType::try_new(&engine, [ValType::I32], [])?);
                let tag1 = Tag::new(&mut store, &tag_ty)?;
                let tag2 = Tag::new(&mut store, &tag_ty)?;
                assert!(Tag::eq(&tag1, &tag1, &store));
                assert!(!Tag::eq(&tag1, &tag2, &store));

                // Checks that the host funcref was initialized.
                let instance = Instance::new_async(&mut store, &module, &imports).await?;
                let run = instance.get_typed_func::<(), i32>(&mut store, "run")?;
                assert_eq!(run.call_async(&mut store, ()).await?, 42);

                Ok(())
            },
        )
        .await
}
