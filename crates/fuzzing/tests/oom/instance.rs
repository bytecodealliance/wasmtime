#![cfg(arc_try_new)]

use wasmtime::{
    Config, Engine, Func, Global, GlobalType, Instance, InstanceAllocationStrategy, Linker, Module,
    Mutability, PoolingAllocationConfig, Result, Store, Val, ValType, WasmBacktrace,
};
use wasmtime_fuzzing::oom::OomTest;

#[test]
fn call_exported_func() -> Result<()> {
    let mut config = Config::new();
    config.concurrency_support(false);
    let engine = Engine::new(&config)?;

    let module = Module::new(
        &engine,
        r#"
            (module
                (func (export "add") (param i32 i32) (result i32)
                    (i32.add (local.get 0) (local.get 1))
                )
            )
        "#,
    )?;
    let linker = Linker::<()>::new(&engine);
    let instance_pre = linker.instantiate_pre(&module)?;

    OomTest::new().test(|| {
        let mut store = Store::try_new(&engine, ())?;
        let instance = instance_pre.instantiate(&mut store)?;
        let add = instance.get_typed_func::<(i32, i32), i32>(&mut store, "add")?;
        let result = add.call(&mut store, (1, 2))?;
        assert_eq!(result, 3);
        Ok(())
    })
}

#[test]
fn instance_new() -> Result<()> {
    let mut config = Config::new();
    config.concurrency_support(false);
    let engine = Engine::new(&config)?;
    let module = Module::new(
        &engine,
        "(module (import \"\" \"f\" (func)) (func (export \"g\") (call 0)))",
    )?;

    OomTest::new().test(|| {
        let mut store = Store::try_new(&engine, ())?;
        let func = Func::try_wrap(&mut store, || {})?;
        let _instance = Instance::new(&mut store, &module, &[func.into()])?;
        Ok(())
    })
}

#[tokio::test]
async fn instance_new_async() -> Result<()> {
    let mut config = Config::new();
    config.concurrency_support(false);
    let engine = Engine::new(&config)?;
    let module = Module::new(
        &engine,
        "(module (import \"\" \"f\" (func)) (func (export \"g\") (call 0)))",
    )?;

    OomTest::new()
        .test_async(|| async {
            let mut store = Store::try_new(&engine, ())?;
            let func = Func::try_wrap(&mut store, || {})?;
            let _instance = Instance::new_async(&mut store, &module, &[func.into()]).await?;
            Ok(())
        })
        .await
}

#[test]
fn instance_get_export() -> Result<()> {
    let mut config = Config::new();
    config.concurrency_support(false);
    let engine = Engine::new(&config)?;
    let module = Module::new(&engine, "(module (func (export \"f\")))")?;
    let linker = Linker::<()>::new(&engine);
    let instance_pre = linker.instantiate_pre(&module)?;

    OomTest::new().test(|| {
        let mut store = Store::try_new(&engine, ())?;
        let instance = instance_pre.instantiate(&mut store)?;
        let _export = instance.get_export(&mut store, "f");
        Ok(())
    })
}

#[test]
fn instance_exports() -> Result<()> {
    let mut config = Config::new();
    config.concurrency_support(false);
    let engine = Engine::new(&config)?;
    let module = Module::new(
        &engine,
        "(module (func (export \"f\")) (memory (export \"m\") 1) (global (export \"g\") i32 (i32.const 0)))",
    )?;
    let linker = Linker::<()>::new(&engine);
    let instance_pre = linker.instantiate_pre(&module)?;

    OomTest::new().test(|| {
        let mut store = Store::try_new(&engine, ())?;
        let instance = instance_pre.instantiate(&mut store)?;
        let count = instance.exports(&mut store).count();
        assert_eq!(count, 3);
        Ok(())
    })
}

#[test]
fn instance_get_func() -> Result<()> {
    let mut config = Config::new();
    config.concurrency_support(false);
    let engine = Engine::new(&config)?;
    let module = Module::new(
        &engine,
        "(module (func (export \"f\") (param i32) (result i32) (local.get 0)))",
    )?;
    let linker = Linker::<()>::new(&engine);
    let instance_pre = linker.instantiate_pre(&module)?;

    OomTest::new().test(|| {
        let mut store = Store::try_new(&engine, ())?;
        let instance = instance_pre.instantiate(&mut store)?;
        let f = instance.get_func(&mut store, "f");
        assert!(f.is_some());
        Ok(())
    })
}

#[test]
fn instance_get_typed_func() -> Result<()> {
    let mut config = Config::new();
    config.concurrency_support(false);
    let engine = Engine::new(&config)?;
    let module = Module::new(
        &engine,
        "(module (func (export \"f\") (param i32) (result i32) (local.get 0)))",
    )?;
    let linker = Linker::<()>::new(&engine);
    let instance_pre = linker.instantiate_pre(&module)?;

    OomTest::new().test(|| {
        let mut store = Store::try_new(&engine, ())?;
        let instance = instance_pre.instantiate(&mut store)?;
        let _f = instance.get_typed_func::<i32, i32>(&mut store, "f")?;
        Ok(())
    })
}

#[test]
fn instance_get_table() -> Result<()> {
    let mut config = Config::new();
    config.concurrency_support(false);
    let engine = Engine::new(&config)?;
    let module = Module::new(&engine, "(module (table (export \"t\") 1 funcref))")?;
    let linker = Linker::<()>::new(&engine);
    let instance_pre = linker.instantiate_pre(&module)?;

    OomTest::new().test(|| {
        let mut store = Store::try_new(&engine, ())?;
        let instance = instance_pre.instantiate(&mut store)?;
        let t = instance.get_table(&mut store, "t");
        assert!(t.is_some());
        Ok(())
    })
}

#[test]
fn instance_get_memory() -> Result<()> {
    let mut config = Config::new();
    config.concurrency_support(false);
    let engine = Engine::new(&config)?;
    let module = Module::new(&engine, "(module (memory (export \"m\") 1))")?;
    let linker = Linker::<()>::new(&engine);
    let instance_pre = linker.instantiate_pre(&module)?;

    OomTest::new().test(|| {
        let mut store = Store::try_new(&engine, ())?;
        let instance = instance_pre.instantiate(&mut store)?;
        let m = instance.get_memory(&mut store, "m");
        assert!(m.is_some());
        Ok(())
    })
}

#[test]
fn instance_get_global() -> Result<()> {
    let mut config = Config::new();
    config.concurrency_support(false);
    let engine = Engine::new(&config)?;
    let module = Module::new(
        &engine,
        "(module (global (export \"g\") i32 (i32.const 42)))",
    )?;
    let linker = Linker::<()>::new(&engine);
    let instance_pre = linker.instantiate_pre(&module)?;

    OomTest::new().test(|| {
        let mut store = Store::try_new(&engine, ())?;
        let instance = instance_pre.instantiate(&mut store)?;
        let g = instance.get_global(&mut store, "g");
        assert!(g.is_some());
        Ok(())
    })
}

#[test]
fn instance_global_init_with_imported_global() -> Result<()> {
    let module_bytes = {
        let mut config = Config::new();
        config.concurrency_support(false);
        let engine = Engine::new(&config)?;
        Module::new(
            &engine,
            r#"
                (module
                    (import "" "g" (global i32))
                    (global (export "g2") (mut i32) (global.get 0))
                )
            "#,
        )?
        .serialize()?
    };
    let mut config = Config::new();
    config.enable_compiler(false);
    config.concurrency_support(false);
    let engine = Engine::new(&config)?;

    OomTest::new().allow_alloc_after_oom(true).test(|| unsafe {
        let module = Module::deserialize(&engine, &module_bytes)?;
        let mut store = Store::try_new(&engine, ())?;
        let g = Global::new(
            &mut store,
            GlobalType::new(ValType::I32, Mutability::Const),
            Val::I32(1000),
        )?;
        let mut linker = Linker::<()>::new(&engine);
        linker.define(&store, "", "g", g)?;
        let instance = linker.instantiate(&mut store, &module)?;
        let g2 = instance.get_global(&mut store, "g2").unwrap();
        assert_eq!(g2.get(&mut store).unwrap_i32(), 1000);
        Ok(())
    })
}

#[test]
fn pooling_allocator_memory_slot_leak_on_oom() -> Result<()> {
    let mut pool = PoolingAllocationConfig::default();
    pool.total_memories(1);
    pool.total_tables(10);
    pool.total_core_instances(10);

    let mut config = Config::new();
    config.concurrency_support(false);
    config.allocation_strategy(InstanceAllocationStrategy::Pooling(pool));
    let engine = Engine::new(&config)?;

    let module = Module::new(&engine, r#"(module (memory (export "m") 1))"#)?;
    let linker = Linker::<()>::new(&engine);
    let instance_pre = linker.instantiate_pre(&module)?;

    // With only one memory slot in the pooling allocator shared amongst all
    // runs here we should be guaranteed that an OOM failure anywhere always
    // returns the memory, if any, back to the pooling allocator.
    OomTest::new().allow_alloc_after_oom(true).test(|| {
        let mut store = Store::try_new(&engine, ())?;
        let _instance = instance_pre.instantiate(&mut store)?;
        Ok(())
    })
}

#[test]
fn table_element_segment_init() -> Result<()> {
    let mut config = Config::new();
    config.concurrency_support(false);
    let engine = Engine::new(&config)?;
    let module = Module::new(
        &engine,
        r#"(module
            (table (export "table") 10 funcref)
            (func $f (result i32) (i32.const 42))
            (elem (i32.const 0) func $f)
        )"#,
    )?;
    let linker = Linker::<()>::new(&engine);
    let instance_pre = linker.instantiate_pre(&module)?;

    OomTest::new().allow_alloc_after_oom(true).test(|| {
        let mut store = Store::try_new(&engine, ())?;
        let _instance = instance_pre.instantiate(&mut store)?;
        Ok(())
    })
}

#[tokio::test]
async fn instance_new_async_reuse_engine() -> Result<()> {
    // A module with an imported function, an exported function, and a rec
    // group with more than one type in it, so that building the context
    // registers several types in the engine's type registry.
    let module_bytes = {
        let mut config = Config::new();
        config.concurrency_support(false);
        let engine = Engine::new(&config)?;
        Module::new(
            &engine,
            r#"
                (module
                    (rec
                        (type $a (func (param (ref null $b)) (result i32)))
                        (type $b (func (param (ref null $a))))
                    )
                    (import "" "f" (func))
                    (func (export "g") (result i32)
                        (call 0)
                        (i32.const 42)
                    )
                )
            "#,
        )?
        .serialize()?
    };

    /// A fresh engine and module for each iteration of the OOM test.
    struct Context {
        engine: Engine,
        module: Module,
    }

    OomTest::new()
        .allow_alloc_after_oom(true)
        .test_with_reuse_async(
            async || {
                let mut config = Config::new();
                config.enable_compiler(false);
                config.concurrency_support(false);
                let engine = Engine::new(&config)?;
                let module = unsafe { Module::deserialize(&engine, &module_bytes)? };
                Ok(Context { engine, module })
            },
            async |cx: &mut Context| {
                let mut store = Store::try_new(&cx.engine, ())?;
                let func = Func::try_wrap(&mut store, || {})?;
                let _ = Instance::new_async(&mut store, &cx.module, &[func.into()]).await?;
                Ok(())
            },
            async |cx: Context| {
                // Whether or not we hit an OOM above, we must still be able to
                // instantiate the module and call into it.
                let mut store = Store::try_new(&cx.engine, ())?;
                let func = Func::try_wrap(&mut store, || {})?;
                let instance = Instance::new_async(&mut store, &cx.module, &[func.into()]).await?;
                let g = instance.get_typed_func::<(), i32>(&mut store, "g")?;
                assert_eq!(g.call_async(&mut store, ()).await?, 42);
                Ok(())
            },
        )
        .await
}

/// Check that an OOM partway through registering a module leaves the store's
/// `ModuleRegistry` consistent.
///
/// Registration patches the `wasm_call` hole in host funcrefs created
/// *earlier*, hence the context's host function is made before the OOM.
#[tokio::test]
async fn module_registry_reuse_store() -> Result<()> {
    let mut config = Config::new();
    config.concurrency_support(false);
    let engine = Engine::new(&config)?;

    // One per instantiation API, so each registration is a fresh insertion
    // rather than a hit on the memoized `store_code` entry.
    let module = |n: u32| -> Result<Module> {
        Module::new(
            &engine,
            &format!(
                r#"
                    (module
                        (import "host" "f" (func $f (result i32)))
                        (func (export "call_host") (result i32)
                            (i32.add (call $f) (i32.const {n}))
                        )
                        (func (export "boom") unreachable)
                    )
                "#
            ),
        )
    };
    let modules: [Module; 5] = std::array::from_fn(|n| module(n as u32).unwrap());

    let mut linker = Linker::<()>::new(&engine);
    linker.func_wrap("host", "f", || 100i32)?;
    let instance_pre = linker.instantiate_pre(&modules[1])?;

    struct Context {
        store: Store<()>,
        host_func: Func,
        module_linker: Linker<()>,
    }

    let make_context = async || -> Result<Context> {
        let mut store = Store::try_new(&engine, ())?;
        let host_func = Func::try_wrap(&mut store, || 100i32)?;
        let mut module_linker = Linker::<()>::new(&engine);
        module_linker.func_wrap("host", "f", || 100i32)?;
        Ok(Context {
            store,
            host_func,
            module_linker,
        })
    };

    /// Check that the imported host function is callable from wasm.
    async fn check_instance(store: &mut Store<()>, instance: Instance, n: i32) -> Result<()> {
        let f = instance.get_typed_func::<(), i32>(&mut *store, "call_host")?;
        assert_eq!(f.call_async(&mut *store, ()).await?, 100 + n);
        Ok(())
    }

    OomTest::new()
        .allow_alloc_after_oom(true)
        .test_with_reuse_async(
            async || make_context().await,
            async |cx: &mut Context| {
                let imports = [cx.host_func.into()];

                // Every API that registers code in the `ModuleRegistry`.
                let _ = Instance::new(&mut cx.store, &modules[0], &imports)?;
                let _ = Instance::new_async(&mut cx.store, &modules[1], &imports).await?;
                let _ = instance_pre.instantiate(&mut cx.store)?;
                let _ = instance_pre.instantiate_async(&mut cx.store).await?;
                let _ = linker.instantiate(&mut cx.store, &modules[2])?;
                let _ = linker.instantiate_async(&mut cx.store, &modules[3]).await?;

                cx.module_linker
                    .module_async(&mut cx.store, "m", &modules[4])
                    .await?;

                Ok(())
            },
            async |mut cx: Context| {
                // The host function must have had its hole filled.
                let imports = [cx.host_func.into()];

                let instance = Instance::new(&mut cx.store, &modules[0], &imports)?;
                check_instance(&mut cx.store, instance, 0).await?;

                let instance = Instance::new_async(&mut cx.store, &modules[1], &imports).await?;
                check_instance(&mut cx.store, instance, 1).await?;

                let instance = instance_pre.instantiate(&mut cx.store)?;
                check_instance(&mut cx.store, instance, 1).await?;

                let instance = instance_pre.instantiate_async(&mut cx.store).await?;
                check_instance(&mut cx.store, instance, 1).await?;

                let instance = linker.instantiate(&mut cx.store, &modules[2])?;
                check_instance(&mut cx.store, instance, 2).await?;

                let instance = linker.instantiate_async(&mut cx.store, &modules[3]).await?;
                check_instance(&mut cx.store, instance, 3).await?;

                // Fresh, since the above may already have defined "m".
                let mut module_linker = Linker::<()>::new(&engine);
                module_linker.func_wrap("host", "f", || 100i32)?;
                module_linker
                    .module_async(&mut cx.store, "m", &modules[4])
                    .await?;
                let f = module_linker
                    .get(&mut cx.store, "m", "call_host")
                    .unwrap()
                    .into_func()
                    .unwrap()
                    .typed::<(), i32>(&cx.store)?;
                assert_eq!(f.call_async(&mut cx.store, ()).await?, 104);

                // Frame-info lookup reads the store's registry.
                let boom = instance.get_typed_func::<(), ()>(&mut cx.store, "boom")?;
                let err = boom.call_async(&mut cx.store, ()).await.unwrap_err();
                let trace = err
                    .downcast_ref::<WasmBacktrace>()
                    .expect("trap should have a wasm backtrace");
                assert!(
                    !trace.frames().is_empty(),
                    "backtrace should have at least one frame"
                );

                Ok(())
            },
        )
        .await
}

/// Check that an OOM partway through a pooling allocation returns every slot
/// it took.
///
/// The pools hold exactly one of everything, so a leaked slot fails the
/// post-OOM instantiation.
#[tokio::test]
async fn pooling_allocator_reuse_engine() -> Result<()> {
    let module_bytes = {
        let mut config = Config::new();
        config.concurrency_support(false);
        let engine = Engine::new(&config)?;
        Module::new(
            &engine,
            r#"
                (module
                    (memory (export "m") 1)
                    (table (export "t") 1 funcref)
                    (func (export "f") (result i32) (i32.const 42))
                )
            "#,
        )?
        .serialize()?
    };

    struct Context {
        engine: Engine,
        module: Module,
    }

    let make_context = async || -> Result<Context> {
        let mut pool = PoolingAllocationConfig::default();
        pool.total_memories(1);
        pool.total_tables(1);
        pool.total_core_instances(1);
        pool.total_stacks(1);

        let mut config = Config::new();
        config.enable_compiler(false);
        config.concurrency_support(false);
        config.allocation_strategy(InstanceAllocationStrategy::Pooling(pool));
        let engine = Engine::new(&config)?;
        let module = unsafe { Module::deserialize(&engine, &module_bytes)? };

        // Warm every pool outside the OOM region: on macOS a
        // `std::sync::Mutex` infallibly box-allocates its pthread mutex on
        // first lock, which would abort under an injected OOM.
        let mut store = Store::try_new(&engine, ())?;
        let instance = Instance::new_async(&mut store, &module, &[]).await?;
        let f = instance.get_typed_func::<(), i32>(&mut store, "f")?;
        assert_eq!(f.call_async(&mut store, ()).await?, 42);
        drop(store);

        Ok(Context { engine, module })
    };

    OomTest::new()
        .allow_alloc_after_oom(true)
        .test_with_reuse_async(
            async || make_context().await,
            async |cx: &mut Context| {
                // Every slot taken here must come back on drop.
                let mut store = Store::try_new(&cx.engine, ())?;
                let instance = Instance::new_async(&mut store, &cx.module, &[]).await?;
                let f = instance.get_typed_func::<(), i32>(&mut store, "f")?;
                assert_eq!(f.call_async(&mut store, ()).await?, 42);
                drop(store);

                // Re-allocate the slots just returned.
                let mut store = Store::try_new(&cx.engine, ())?;
                let instance = Instance::new_async(&mut store, &cx.module, &[]).await?;
                let f = instance.get_typed_func::<(), i32>(&mut store, "f")?;
                assert_eq!(f.call_async(&mut store, ()).await?, 42);

                Ok(())
            },
            async |cx: Context| {
                // With room for exactly one of each, this only succeeds if
                // nothing was leaked.
                let mut store = Store::try_new(&cx.engine, ())?;
                let instance = Instance::new_async(&mut store, &cx.module, &[]).await?;
                let f = instance.get_typed_func::<(), i32>(&mut store, "f")?;
                assert_eq!(f.call_async(&mut store, ()).await?, 42);

                let memory = instance.get_memory(&mut store, "m").unwrap();
                assert_eq!(memory.size(&store), 1);
                let table = instance.get_table(&mut store, "t").unwrap();
                assert_eq!(table.size(&store), 1);

                Ok(())
            },
        )
        .await
}
