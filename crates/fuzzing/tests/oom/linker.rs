#![cfg(arc_try_new)]

use wasmtime::{
    Config, Engine, Func, FuncType, Instance, Linker, Module, Result, Store, error::Context as _,
};
use wasmtime_fuzzing::oom::OomTest;

#[test]
fn linker_new() -> Result<()> {
    OomTest::new().test(|| {
        let mut config = Config::new();
        config.enable_compiler(false);
        let engine = Engine::new(&config)?;
        let _linker = Linker::<()>::new(&engine);
        Ok(())
    })
}

#[test]
fn linker_func_wrap() -> Result<()> {
    OomTest::new().test(|| {
        let mut config = Config::new();
        config.enable_compiler(false);
        let engine = Engine::new(&config)?;
        let mut linker = Linker::<()>::new(&engine);
        linker.func_wrap("module", "func", |x: i32| x * 2)?;
        Ok(())
    })
}

#[test]
fn linker_instantiate_pre() -> Result<()> {
    let mut config = Config::new();
    config.concurrency_support(false);

    let engine = Engine::new(&config)?;

    let mut linker = Linker::<()>::new(&engine);
    linker.func_wrap("module", "func", |x: i32| x * 2)?;

    let module = Module::new(
        &engine,
        r#"
            (module
                (import "module" "func" (func (param i32) (result i32)))

                (memory (export "memory") 1)
                (data (i32.const 0) "a")

                (table (export "table") 1 funcref)
                (elem (i32.const 0) func 1)

                (func (export "func") (param i32) (result i32)
                    (call 0 (local.get 0))
                )
            )
        "#,
    )?;

    OomTest::new().test(|| {
        let _ = linker.instantiate_pre(&module)?;
        Ok(())
    })
}

#[tokio::test]
async fn linker_instantiate_async() -> Result<()> {
    let mut config = Config::new();
    config.concurrency_support(false);

    let engine = Engine::new(&config)?;

    let mut linker = Linker::<()>::new(&engine);
    linker.func_wrap("module", "func", |x: i32| x * 2)?;

    let module = Module::new(
        &engine,
        r#"
            (module
                (import "module" "func" (func (param i32) (result i32)))

                (memory (export "memory") 1)
                (data (i32.const 0) "a")

                (table (export "table") 1 funcref)
                (elem (i32.const 0) func 1)

                (func (export "func") (param i32) (result i32)
                    (call 0 (local.get 0))
                )
            )
        "#,
    )?;

    OomTest::new()
        .test_async(|| async {
            let mut store = Store::try_new(&engine, ())?;
            let _instance = linker.instantiate_async(&mut store, &module).await?;
            Ok(())
        })
        .await
}

#[test]
fn linker_define() -> Result<()> {
    let mut config = Config::new();
    config.enable_compiler(false);
    config.concurrency_support(false);
    let engine = Engine::new(&config)?;

    OomTest::new().test(|| {
        let mut store = Store::try_new(&engine, ())?;
        let mut linker = Linker::<()>::new(&engine);
        let func = Func::try_wrap(&mut store, || {})?;
        linker.define(&store, "mod", "func", func)?;
        Ok(())
    })
}

#[test]
fn linker_func_new() -> Result<()> {
    let mut config = Config::new();
    config.enable_compiler(false);
    config.concurrency_support(false);
    let engine = Engine::new(&config)?;

    OomTest::new().test(|| {
        let mut linker = Linker::<()>::new(&engine);
        linker.func_new(
            "mod",
            "func",
            FuncType::try_new(&engine, [], [])?,
            |_caller, _params, _results| Ok(()),
        )?;
        Ok(())
    })
}

#[test]
fn linker_instance() -> Result<()> {
    let mut config = Config::new();
    config.concurrency_support(false);
    let engine = Engine::new(&config)?;
    let module = Module::new(
        &engine,
        r#"(module (func (export "f")) (memory (export "m") 1))"#,
    )?;
    let pre_linker = Linker::<()>::new(&engine);
    let instance_pre = pre_linker.instantiate_pre(&module)?;

    OomTest::new().test(|| {
        let mut store = Store::try_new(&engine, ())?;
        let instance = instance_pre.instantiate(&mut store)?;
        let mut linker = Linker::<()>::new(&engine);
        linker.instance(&mut store, "inst", instance)?;
        Ok(())
    })
}

#[test]
fn linker_get() -> Result<()> {
    let mut config = Config::new();
    config.enable_compiler(false);
    config.concurrency_support(false);
    let engine = Engine::new(&config)?;

    let mut linker = Linker::<()>::new(&engine);
    linker.func_wrap("mod", "func", || {})?;

    OomTest::new().test(|| {
        let mut store = Store::try_new(&engine, ())?;
        let _item = linker.get(&mut store, "mod", "func")?;
        Ok(())
    })
}

#[test]
fn linker_get_then_instantiate() -> Result<()> {
    let mut config = Config::new();
    config.concurrency_support(false);
    let engine = Engine::new(&config)?;
    let module = Module::new(&engine, "(module)")?;

    OomTest::new()
        .allow_alloc_after_oom(true)
        .alloc_succeeds_after_oom(true)
        .test(|| {
            let mut store = Store::try_new(&engine, ())?;
            let mut linker = Linker::<()>::new(&engine);
            linker.func_wrap("mod", "func", || {})?;
            let item = linker.get(&mut store, "mod", "func");
            // Drop the linker so the store holds the only remaining reference
            // to the host function, if any. Whether or not `get` hit OOM, the
            // store must still be usable afterwards.
            drop(linker);
            Instance::new(&mut store, &module, &[])?;
            item?;
            Ok(())
        })
}

#[test]
fn linker_get_by_import() -> Result<()> {
    let mut config = Config::new();
    config.concurrency_support(false);
    let engine = Engine::new(&config)?;
    let module = Module::new(&engine, r#"(module (import "mod" "func" (func)))"#)?;

    let mut linker = Linker::<()>::new(&engine);
    linker.func_wrap("mod", "func", || {})?;

    let import = module.imports().next().unwrap();

    OomTest::new().test(|| {
        let mut store = Store::try_new(&engine, ())?;
        let ext = linker.try_get_by_import(&mut store, &import)?;
        assert!(ext.is_some());
        Ok(())
    })
}

#[test]
fn linker_get_default() -> Result<()> {
    let mut config = Config::new();
    config.concurrency_support(false);
    let engine = Engine::new(&config)?;
    let module = Module::new(&engine, r#"(module (func (export "_start")))"#)?;
    let linker = Linker::<()>::new(&engine);
    let instance_pre = linker.instantiate_pre(&module)?;

    OomTest::new().test(|| {
        let mut store = Store::try_new(&engine, ())?;
        let instance = instance_pre.instantiate(&mut store)?;
        let mut linker = Linker::<()>::new(&engine);
        linker.instance(&mut store, "inst", instance)?;
        let _default = linker.get_default(&mut store, "inst")?;
        Ok(())
    })
}

#[tokio::test]
async fn linker_module_async() -> Result<()> {
    let mut config = Config::new();
    config.concurrency_support(false);
    let engine = Engine::new(&config)?;
    let module = Module::new(
        &engine,
        r#"(module (func (export "f")) (memory (export "m") 1))"#,
    )?;

    OomTest::new()
        .test_async(|| async {
            let mut store = Store::try_new(&engine, ())?;
            let mut linker = Linker::<()>::new(&engine);
            linker.module_async(&mut store, "m", &module).await?;
            Ok(())
        })
        .await
}

// Note: linker_define_unknown_imports_as_traps and
// linker_define_unknown_imports_as_default_values are not tested under OOM
// because UnknownImportError::new uses infallible String allocations
// (to_string) that cannot be made fallible without changing the public API.

#[test]
fn linker_alias() -> Result<()> {
    let mut config = Config::new();
    config.enable_compiler(false);
    config.concurrency_support(false);
    let engine = Engine::new(&config)?;

    OomTest::new().test(|| {
        let mut linker = Linker::<()>::new(&engine);
        linker.func_wrap("mod", "func", || {})?;
        linker.alias("mod", "func", "mod2", "func2")?;
        Ok(())
    })
}

#[test]
fn linker_alias_module() -> Result<()> {
    let mut config = Config::new();
    config.enable_compiler(false);
    config.concurrency_support(false);
    let engine = Engine::new(&config)?;

    OomTest::new().test(|| {
        let mut linker = Linker::<()>::new(&engine);
        linker.func_wrap("mod", "func1", || {})?;
        linker.func_wrap("mod", "func2", || {})?;
        linker.alias_module("mod", "mod2")?;
        Ok(())
    })
}

/// Check that an OOM partway through pushing host functions leaves the
/// store's `FuncRefs` arena consistent.
///
/// The `Linker` getters look like reads but mutate the store, and a funcref
/// whose `wasm_call` hole never gets filled only shows up when wasm calls it.
#[tokio::test]
async fn func_refs_reuse_store() -> Result<()> {
    let mut config = Config::new();
    config.concurrency_support(false);
    let engine = Engine::new(&config)?;

    let module = Module::new(
        &engine,
        r#"
            (module
                (import "mod" "func" (func $f (result i32)))
                (func (export "_start") (result i32) (call $f))
            )
        "#,
    )?;

    let mut linker = Linker::<()>::new(&engine);
    linker.func_wrap("mod", "func", || 42i32)?;
    let instance_pre = linker.instantiate_pre(&module)?;
    let import = module.imports().next().unwrap();

    OomTest::new()
        .allow_alloc_after_oom(true)
        .test_with_reuse_async(
            async || Store::try_new(&engine, ()),
            async |store: &mut Store<()>| {
                // Every API that pushes into the `FuncRefs` arena.
                let _ = Func::try_wrap(&mut *store, || 42i32)?;
                let _ = linker.get(&mut *store, "mod", "func")?;
                let _ = linker.try_get_by_import(&mut *store, &import)?;
                let _ = instance_pre.instantiate(&mut *store)?;
                let _ = instance_pre.instantiate_async(&mut *store).await?;
                Ok(())
            },
            async |mut store: Store<()>| {
                // Every funcref must be callable from wasm.
                let func = Func::try_wrap(&mut store, || 42i32)?;
                let instance = Instance::new(&mut store, &module, &[func.into()])?;
                let start = instance.get_typed_func::<(), i32>(&mut store, "_start")?;
                assert_eq!(start.call(&mut store, ())?, 42);

                let got = linker
                    .get(&mut store, "mod", "func")
                    .unwrap()
                    .into_func()
                    .unwrap();
                let instance = Instance::new(&mut store, &module, &[got.into()])?;
                let start = instance.get_typed_func::<(), i32>(&mut store, "_start")?;
                assert_eq!(start.call(&mut store, ())?, 42);

                let by_import = linker
                    .try_get_by_import(&mut store, &import)?
                    .expect("import should be satisfied")
                    .into_func()
                    .unwrap();
                assert_eq!(
                    by_import.typed::<(), i32>(&store)?.call(&mut store, ())?,
                    42
                );

                let instance = instance_pre.instantiate(&mut store)?;
                let start = instance.get_typed_func::<(), i32>(&mut store, "_start")?;
                assert_eq!(start.call(&mut store, ())?, 42);

                let instance = instance_pre.instantiate_async(&mut store).await?;
                let start = instance.get_typed_func::<(), i32>(&mut store, "_start")?;
                assert_eq!(start.call_async(&mut store, ()).await?, 42);

                // `get_default` reads the arena too, via `get`.
                let mut linker = Linker::<()>::new(&engine);
                linker.instance(&mut store, "inst", instance)?;
                let default = linker.get_default(&mut store, "inst")?;
                assert_eq!(default.typed::<(), i32>(&store)?.call(&mut store, ())?, 42);

                Ok(())
            },
        )
        .await
}

/// Check that an OOM partway through defining items leaves a `Linker`'s
/// string pool and map consistent.
///
/// `import_key` interns into the pool *before* `insert` touches the map, so
/// an OOM between them orphans an atom.
#[tokio::test]
async fn linker_state_reuse_linker() -> Result<()> {
    let mut config = Config::new();
    config.concurrency_support(false);
    let engine = Engine::new(&config)?;

    let module = Module::new(
        &engine,
        r#"(module (func (export "f") (result i32) (i32.const 42)) (memory (export "m") 1))"#,
    )?;
    let importing_module = Module::new(
        &engine,
        r#"
            (module
                (import "post" "wrapped" (func $f (result i32)))
                (func (export "_start") (result i32) (call $f))
            )
        "#,
    )?;
    let import = importing_module.imports().next().unwrap();

    // `Linker::instance` requires an instance from the store it is given.
    let instance_pre = Linker::<()>::new(&engine).instantiate_pre(&module)?;

    OomTest::new()
        .allow_alloc_after_oom(true)
        .test_with_reuse_async(
            async || Ok(Linker::<()>::new(&engine)),
            async |linker: &mut Linker<()>| {
                // Every API that interns into the pool and inserts into the
                // map.
                linker.func_wrap("mod", "wrapped", || 42i32)?;
                linker.func_new(
                    "mod",
                    "new",
                    FuncType::try_new(&engine, [], [])?,
                    |_caller, _params, _results| Ok(()),
                )?;

                let mut store = Store::try_new(&engine, ())?;
                let func = Func::try_wrap(&mut store, || {})?;
                linker.define(&store, "mod", "defined", func)?;
                let instance = instance_pre.instantiate(&mut store)?;
                linker.instance(&mut store, "inst", instance)?;
                linker.module_async(&mut store, "m", &module).await?;

                linker.alias("mod", "wrapped", "mod2", "aliased")?;
                linker.alias_module("mod", "mod3")?;

                Ok(())
            },
            async |mut linker: Linker<()>| {
                let mut store = Store::try_new(&engine, ())?;

                // A fresh namespace: the test function may already have
                // defined these.
                linker.func_wrap("post", "wrapped", || 42i32)?;
                linker.alias("post", "wrapped", "post2", "aliased")?;
                linker.alias_module("post", "post3")?;

                for (module, name) in [
                    ("post", "wrapped"),
                    ("post2", "aliased"),
                    ("post3", "wrapped"),
                ] {
                    let func = linker
                        .get(&mut store, module, name)
                        .with_context(|| format!("{module}::{name} should be defined"))?
                        .into_func()
                        .unwrap();
                    assert_eq!(func.typed::<(), i32>(&store)?.call(&mut store, ())?, 42);
                }

                let instance = instance_pre.instantiate(&mut store)?;
                linker.instance(&mut store, "post_inst", instance)?;
                let f = linker
                    .get(&mut store, "post_inst", "f")?
                    .into_func()
                    .unwrap();
                assert_eq!(f.typed::<(), i32>(&store)?.call(&mut store, ())?, 42);

                linker.module_async(&mut store, "post_m", &module).await?;
                let f = linker.get(&mut store, "post_m", "f")?.into_func().unwrap();
                assert_eq!(f.typed::<(), i32>(&store)?.call(&mut store, ())?, 42);

                let ext = linker
                    .try_get_by_import(&mut store, &import)?
                    .expect("import should be satisfied");
                assert!(ext.into_func().is_some());

                let instance = linker.instantiate(&mut store, &importing_module)?;
                let start = instance.get_typed_func::<(), i32>(&mut store, "_start")?;
                assert_eq!(start.call(&mut store, ())?, 42);

                Ok(())
            },
        )
        .await
}
