use wasmtime::component::{
    Accessor, Component, FutureReader, Linker, ResourceTableError, StreamReader,
};
use wasmtime::{AsContextMut, Config, Engine, Result, Store, StoreContextMut};

const TASK_ENTRIES: usize = if cfg!(feature = "task-group-hook") {
    4
} else {
    3
};

fn engine() -> Result<Engine> {
    let mut config = Config::new();
    config.wasm_component_model_async(true);
    Engine::new(&config)
}

fn len<T>(store: &mut Store<T>) -> usize {
    store.concurrent_resource_table().unwrap().len()
}

fn capacity<T>(store: &mut Store<T>) -> usize {
    store.concurrent_resource_table().unwrap().max_capacity()
}

fn set_capacity<T>(store: &mut Store<T>, capacity: usize) {
    store
        .concurrent_resource_table()
        .unwrap()
        .set_max_capacity(capacity);
}

fn assert_full(error: wasmtime::Error) {
    assert!(
        matches!(
            error.downcast_ref::<ResourceTableError>(),
            Some(ResourceTableError::Full)
        ),
        "{error:?}"
    );
}

#[test]
fn configuration_and_context_forwarding() -> Result<()> {
    let engine = engine()?;
    let mut store = Store::new(&engine, ());
    assert_eq!(len(&mut store), 0);
    assert_eq!(capacity(&mut store), 1_000_000);
    set_capacity(&mut store, 0);
    {
        let mut context = store.as_context_mut();
        assert_eq!(context.concurrent_resource_table().unwrap().len(), 0);
        assert_eq!(
            context.concurrent_resource_table().unwrap().max_capacity(),
            0
        );
        context
            .concurrent_resource_table()
            .unwrap()
            .set_max_capacity(3);
    }
    assert_eq!(capacity(&mut store), 3);

    let mut reader = StreamReader::new(&mut store, Vec::<u8>::new())?;
    assert_eq!(len(&mut store), 3);
    reader.close(&mut store)?;
    assert_eq!(len(&mut store), 0);

    set_capacity(&mut store, 3);
    set_capacity(&mut store, 4);
    assert_eq!(capacity(&mut store), 4);

    let mut other = Store::new(&engine, ());
    assert_eq!(capacity(&mut other), 1_000_000);
    Ok(())
}

#[test]
fn configuration_requires_concurrency_support() -> Result<()> {
    let mut config = Config::new();
    config.concurrency_support(false);
    let engine = Engine::new(&config)?;
    let mut store = Store::new(&engine, ());
    assert!(store.concurrent_resource_table().is_none());
    let mut context = store.as_context_mut();
    assert!(context.concurrent_resource_table().is_none());
    Ok(())
}

#[test]
fn host_transmits_refuse_without_retaining_partial_allocations() -> Result<()> {
    let engine = engine()?;
    for capacity in 0..3 {
        let mut store = Store::new(&engine, ());
        set_capacity(&mut store, capacity);
        for _ in 0..2 {
            assert_full(StreamReader::new(&mut store, Vec::<u8>::new()).unwrap_err());
            assert_eq!(len(&mut store), 0);
            assert_full(
                FutureReader::new(&mut store, async { wasmtime::error::Ok(0_u8) }).unwrap_err(),
            );
            assert_eq!(len(&mut store), 0);
        }
        set_capacity(&mut store, 3);
        for _ in 0..2 {
            let mut stream = StreamReader::new(&mut store, Vec::<u8>::new())?;
            assert_eq!(len(&mut store), 3);
            assert_full(
                FutureReader::new(&mut store, async { wasmtime::error::Ok(0_u8) }).unwrap_err(),
            );
            assert_eq!(len(&mut store), 3);
            stream.close(&mut store)?;
            assert_eq!(len(&mut store), 0);
            let mut future = FutureReader::new(&mut store, async { wasmtime::error::Ok(0_u8) })?;
            assert_eq!(len(&mut store), 3);
            future.close(&mut store)?;
            assert_eq!(len(&mut store), 0);
        }
    }
    Ok(())
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn async_export_needs_all_task_entries_before_starting() -> Result<()> {
    let engine = engine()?;
    let component = Component::new(
        &engine,
        r#"(component
            (core module $m (func (export "run")))
            (core instance $i (instantiate $m))
            (func (export "run") async
                (canon lift (core func $i "run")))
        )"#,
    )?;
    let linker = Linker::new(&engine);
    for capacity in 0..TASK_ENTRIES {
        let mut store = Store::new(&engine, ());
        set_capacity(&mut store, capacity);
        let instance = linker.instantiate_async(&mut store, &component).await?;
        let run = instance.get_typed_func::<(), ()>(&mut store, "run")?;
        for _ in 0..2 {
            let error = match run.start_call_concurrent(&mut store, ()) {
                Ok(_) => panic!("export must refuse with too few task slots"),
                Err(error) => error,
            };
            assert_full(error);
            assert_eq!(len(&mut store), 0);
        }
        set_capacity(&mut store, TASK_ENTRIES);
        for _ in 0..2 {
            let call = run.start_call_concurrent(&mut store, ())?;
            assert_eq!(len(&mut store), TASK_ENTRIES);
            store
                .run_concurrent(async |accessor| run.finish_call_concurrent(accessor, call).await)
                .await??;
            assert_eq!(len(&mut store), 0);
        }
    }
    Ok(())
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn host_import_admission_precedes_host_work() -> Result<()> {
    let engine = engine()?;
    let component = Component::new(
        &engine,
        r#"(component
            (import "f" (func $f async))
            (core func $f (canon lower (func $f)))
            (core module $m
                (import "" "f" (func $f))
                (func (export "run") call $f))
            (core instance $i (instantiate $m
                (with "" (instance (export "f" (func $f))))))
            (func (export "run") async
                (canon lift (core func $i "run")))
        )"#,
    )?;
    let mut linker = Linker::new(&engine);
    linker
        .root()
        .func_wrap_concurrent("f", |accessor: &Accessor<usize>, (): ()| {
            Box::pin(async move {
                accessor.with(|mut access| -> Result<()> {
                    let mut context = access.as_context_mut();
                    let table = context.concurrent_resource_table().unwrap();
                    // The current host task is deferred until this future
                    // yields, so admission reserves its prospective slot.
                    if table.len() >= table.max_capacity() {
                        return Err(ResourceTableError::Full.into());
                    }
                    *access.get() += 1;
                    Ok(())
                })?;
                tokio::task::yield_now().await;
                accessor.with(|mut access| {
                    assert_eq!(
                        access
                            .as_context_mut()
                            .concurrent_resource_table()
                            .unwrap()
                            .len(),
                        TASK_ENTRIES + 1
                    );
                });
                Ok(())
            })
        })?;
    for capacity in [TASK_ENTRIES, TASK_ENTRIES + 1] {
        let mut store = Store::new(&engine, 0_usize);
        set_capacity(&mut store, capacity);
        let instance = linker.instantiate_async(&mut store, &component).await?;
        let run = instance.get_typed_func::<(), ()>(&mut store, "run")?;
        let result = store
            .run_concurrent(async |accessor| run.call_concurrent(accessor, ()).await)
            .await
            .and_then(|result| result);
        if capacity == TASK_ENTRIES {
            assert_full(result.unwrap_err());
            assert_eq!(*store.data(), 0);
        } else {
            result?;
            assert_eq!(*store.data(), 1);
            assert_eq!(len(&mut store), 0);
            store
                .run_concurrent(async |accessor| run.call_concurrent(accessor, ()).await)
                .await??;
            assert_eq!(*store.data(), 2);
        }
    }
    Ok(())
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn sync_host_import_runs_at_the_bound() -> Result<()> {
    let engine = engine()?;
    let component = Component::new(
        &engine,
        r#"(component
            (import "f" (func $f))
            (core func $f (canon lower (func $f)))
            (core module $m
                (import "" "f" (func $f))
                (func (export "run") call $f))
            (core instance $i (instantiate $m
                (with "" (instance (export "f" (func $f))))))
            (func (export "run") async
                (canon lift (core func $i "run")))
        )"#,
    )?;
    let mut linker = Linker::new(&engine);
    linker
        .root()
        .func_wrap("f", |mut context: StoreContextMut<usize>, (): ()| {
            assert_eq!(
                context.concurrent_resource_table().unwrap().len(),
                TASK_ENTRIES
            );
            *context.data_mut() += 1;
            Ok(())
        })?;
    let mut store = Store::new(&engine, 0_usize);
    set_capacity(&mut store, TASK_ENTRIES);
    let instance = linker.instantiate_async(&mut store, &component).await?;
    let run = instance.get_typed_func::<(), ()>(&mut store, "run")?;
    store
        .run_concurrent(async |accessor| run.call_concurrent(accessor, ()).await)
        .await??;
    assert_eq!(*store.data(), 1);
    assert_eq!(len(&mut store), 0);
    Ok(())
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn guest_transmits_share_the_store_capacity() -> Result<()> {
    let engine = engine()?;
    for kind in ["future", "stream"] {
        let (write_definition, write_import, write_value) = if kind == "future" {
            (
                r#"(core module $memory (memory (export "memory") 1))
                    (core instance $memory (instantiate $memory))
                    (core func $write (canon future.write $t async
                        (memory (core memory $memory "memory"))))
                    (core func $cancel-write (canon future.cancel-write $t))"#,
                r#"(import "" "write" (func $write (param i32 i32) (result i32)))
                    (import "" "cancel-write" (func $cancel-write (param i32) (result i32)))"#,
                r#"(call $write
                    (i32.wrap_i64 (i64.shr_u (local.get $pair) (i64.const 32)))
                    (i32.const 0))
                    drop"#,
            )
        } else {
            ("", "", "")
        };
        let write_export = if kind == "future" {
            r#"(export "write" (func $write))
                (export "cancel-write" (func $cancel-write))"#
        } else {
            ""
        };
        let finish_write = if kind == "future" {
            r#"(call $cancel-write
                (i32.wrap_i64 (i64.shr_u (local.get $pair) (i64.const 32))))
                drop"#
        } else {
            ""
        };
        let component = Component::new(
            &engine,
            format!(
                r#"(component
                    (type $t ({kind}))
                    (import "observe" (func $observe))
                    (core func $observe (canon lower (func $observe)))
                    (core func $new (canon {kind}.new $t))
                    (core func $drop-read (canon {kind}.drop-readable $t))
                    (core func $drop-write (canon {kind}.drop-writable $t))
                    {write_definition}
                    (core module $m
                        (import "" "observe" (func $observe))
                        (import "" "new" (func $new (result i64)))
                        (import "" "drop-read" (func $drop-read (param i32)))
                        (import "" "drop-write" (func $drop-write (param i32)))
                        {write_import}
                        (func (export "run") (local $pair i64)
                            call $new
                            local.set $pair
                            call $observe
                            {write_value}
                            (call $drop-read (i32.wrap_i64 (local.get $pair)))
                            {finish_write}
                            (call $drop-write
                                (i32.wrap_i64 (i64.shr_u (local.get $pair) (i64.const 32))))
                            call $observe))
                    (core instance $i (instantiate $m
                        (with "" (instance
                            (export "observe" (func $observe))
                            (export "new" (func $new))
                            (export "drop-read" (func $drop-read))
                            {write_export}
                            (export "drop-write" (func $drop-write))))))
                    (func (export "run") async
                        (canon lift (core func $i "run")))
                )"#
            ),
        )?;
        let mut linker = Linker::new(&engine);
        linker.root().func_wrap(
            "observe",
            |mut context: StoreContextMut<Vec<usize>>, (): ()| {
                let count = context.concurrent_resource_table().unwrap().len();
                context.data_mut().push(count);
                Ok(())
            },
        )?;
        for capacity in TASK_ENTRIES..TASK_ENTRIES + 3 {
            let mut store = Store::new(&engine, Vec::new());
            set_capacity(&mut store, capacity);
            let instance = linker.instantiate_async(&mut store, &component).await?;
            let run = instance.get_typed_func::<(), ()>(&mut store, "run")?;
            let error = store
                .run_concurrent(async |accessor| run.call_concurrent(accessor, ()).await)
                .await
                .and_then(|result| result)
                .unwrap_err();
            assert_full(error);
            assert!(store.data().is_empty());
            assert!(
                len(&mut store) <= TASK_ENTRIES,
                "failed pair allocation retained slots"
            );
        }
        let mut store = Store::new(&engine, Vec::new());
        set_capacity(&mut store, TASK_ENTRIES + 3);
        let instance = linker.instantiate_async(&mut store, &component).await?;
        let run = instance.get_typed_func::<(), ()>(&mut store, "run")?;
        for _ in 0..2 {
            store
                .run_concurrent(async |accessor| run.call_concurrent(accessor, ()).await)
                .await??;
            assert_eq!(len(&mut store), 0);
        }
        assert_eq!(
            store.data(),
            &[
                TASK_ENTRIES + 3,
                TASK_ENTRIES,
                TASK_ENTRIES + 3,
                TASK_ENTRIES
            ]
        );
    }
    Ok(())
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn guest_waitable_sets_share_the_store_capacity() -> Result<()> {
    let engine = engine()?;
    let component = Component::new(
        &engine,
        r#"(component
            (import "observe" (func $observe))
            (core func $observe (canon lower (func $observe)))
            (core func $new (canon waitable-set.new))
            (core func $drop (canon waitable-set.drop))
            (core module $m
                (import "" "observe" (func $observe))
                (import "" "new" (func $new (result i32)))
                (import "" "drop" (func $drop (param i32)))
                (func (export "run") (local $set i32)
                    call $new
                    local.set $set
                    call $observe
                    local.get $set
                    call $drop))
            (core instance $i (instantiate $m
                (with "" (instance
                    (export "observe" (func $observe))
                    (export "new" (func $new))
                    (export "drop" (func $drop))))))
            (func (export "run") async
                (canon lift (core func $i "run")))
        )"#,
    )?;
    let mut linker = Linker::new(&engine);
    linker
        .root()
        .func_wrap("observe", |mut context: StoreContextMut<usize>, (): ()| {
            assert_eq!(
                context.concurrent_resource_table().unwrap().len(),
                TASK_ENTRIES + 1
            );
            *context.data_mut() += 1;
            Ok(())
        })?;
    for capacity in [TASK_ENTRIES, TASK_ENTRIES + 1] {
        let mut store = Store::new(&engine, 0_usize);
        set_capacity(&mut store, capacity);
        let instance = linker.instantiate_async(&mut store, &component).await?;
        let run = instance.get_typed_func::<(), ()>(&mut store, "run")?;
        let result = store
            .run_concurrent(async |accessor| run.call_concurrent(accessor, ()).await)
            .await
            .and_then(|result| result);
        if capacity == TASK_ENTRIES {
            assert_full(result.unwrap_err());
            assert_eq!(*store.data(), 0);
        } else {
            result?;
            assert_eq!(*store.data(), 1);
            assert_eq!(len(&mut store), 0);
            store
                .run_concurrent(async |accessor| run.call_concurrent(accessor, ()).await)
                .await??;
            assert_eq!(*store.data(), 2);
        }
    }
    Ok(())
}
