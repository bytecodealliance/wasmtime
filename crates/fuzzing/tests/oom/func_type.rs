#![cfg(arc_try_new)]

use std::iter;
use wasmtime::component::{Component, Linker};
use wasmtime::{
    Config, Engine, Finality, Func, FuncType, Module, Result, Store, Tag, TagType, ValType,
    WasmBacktrace,
};
use wasmtime_fuzzing::oom::OomTest;

#[test]
fn func_type_try_new() -> Result<()> {
    let mut config = Config::new();
    config.enable_compiler(false);
    let engine = Engine::new(&config)?;

    // Run this OOM test a few times to make sure that we leave the engine's
    // type registry in a good state when failing to register new types.
    for i in 1..6 {
        OomTest::new().test(|| {
            let ty1 = FuncType::try_new(
                &engine,
                iter::repeat(ValType::ANYREF).take(i),
                iter::repeat(ValType::ANYREF).take(i),
            )?;
            assert_eq!(ty1.params().len(), i);
            assert_eq!(ty1.results().len(), i);

            let ty2 = FuncType::try_new(
                &engine,
                iter::repeat(ValType::ANYREF).take(i),
                iter::repeat(ValType::ANYREF).take(i),
            )?;
            assert_eq!(ty2.params().len(), i);
            assert_eq!(ty2.results().len(), i);

            let ty3 = FuncType::try_new(&engine, [], [])?;
            assert_eq!(ty3.params().len(), 0);
            assert_eq!(ty3.results().len(), 0);

            assert!(
                !FuncType::eq(&ty2, &ty3),
                "{ty2:?} should not be equal to {ty3:?}"
            );

            Ok(())
        })?;
    }

    Ok(())
}

/// Check that an OOM partway through registering types leaves the engine's
/// type registry consistent.
///
/// This covers every way the registry is mutated: the `FuncType`
/// constructors, `Tag::new`, host functions, and the bulk registration that
/// `Module::deserialize` and `Component::deserialize` perform.
/// `Component::deserialize` also populates the global code registry.
#[test]
fn type_registry_reuse_engine() -> Result<()> {
    // A rec group registers a cycle of types in one go.
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
                    (type $c (func (param i32 i32) (result i64)))
                    (func (export "c") (type $c) (i64.const 0))
                    (func (export "b") (type $b))
                )
            "#,
        )?
        .serialize()?
    };

    // The realloc-using lift forces a second, separate registration.
    let component_bytes = {
        let mut config = Config::new();
        config.concurrency_support(false);
        let engine = Engine::new(&config)?;
        Component::new(
            &engine,
            r#"
                (component
                    (core module $libc
                        (memory (export "memory") 1)
                        (func (export "realloc") (param i32 i32 i32 i32) (result i32)
                            (i32.const 8)
                        )
                    )
                    (core instance $libc (instantiate $libc))
                    (core module $m
                        (func (export "id") (param i32) (result i32) (local.get 0))
                        (func (export "boom") unreachable)
                    )
                    (core instance $i (instantiate $m))
                    (func (export "id") (param "x" s32) (result s32)
                        (canon lift (core func $i "id"))
                    )
                    (func (export "boom")
                        (canon lift (core func $i "boom")
                            (memory (core memory $libc "memory"))
                            (realloc (core func $libc "realloc")))
                    )
                )
            "#,
        )?
        .serialize()?
    };

    OomTest::new()
        // Error propagation via anyhow allocates after OOM.
        .allow_alloc_after_oom(true)
        .test_with_reuse(
            || {
                let mut config = Config::new();
                config.enable_compiler(false);
                config.concurrency_support(false);
                config.wasm_exceptions(true);
                Engine::new(&config)
            },
            |engine: &mut Engine| {
                let mut store = Store::try_new(&*engine, ())?;

                // Subtyping forces a check against a just-registered entry.
                let super_ty = FuncType::with_finality_and_supertype(
                    engine,
                    Finality::NonFinal,
                    None,
                    [ValType::I32],
                    [ValType::I64],
                )?;
                let _sub_ty = FuncType::with_finality_and_supertype(
                    engine,
                    Finality::Final,
                    Some(&super_ty),
                    [ValType::I32],
                    [ValType::I64],
                )?;

                // `Tag::new` registers an exception type too; an OOM
                // between the two must unwind the first.
                let tag_ty = TagType::new(FuncType::try_new(engine, [ValType::F32], [])?);
                let _tag = Tag::new(&mut store, &tag_ty)?;

                // A host function registers its signature in the engine too,
                // claiming a type index ahead of the module's and component's
                // own types and so shifting those past the end of the space
                // reserved for their trampolines.
                let _func = Func::try_wrap(&mut store, |x: i32| x * 2)?;

                // Bulk registration of a whole module's and component's types.
                unsafe {
                    let _ = Module::deserialize(&*engine, &module_bytes)?;
                    let _ = Component::deserialize(&*engine, &component_bytes)?;
                }

                Ok(())
            },
            |engine: Engine| {
                let mut store = Store::try_new(&engine, ())?;

                let a = FuncType::try_new(&engine, [ValType::I32], [ValType::I64])?;
                let b = FuncType::try_new(&engine, [ValType::I32], [ValType::I64])?;
                let c = FuncType::try_new(&engine, [ValType::F32], [])?;
                assert!(FuncType::eq(&a, &b), "{a:?} should be equal to {b:?}");
                assert!(!FuncType::eq(&a, &c), "{a:?} should not be equal to {c:?}");

                let super_ty = FuncType::with_finality_and_supertype(
                    &engine,
                    Finality::NonFinal,
                    None,
                    [ValType::I32],
                    [ValType::I64],
                )?;
                let sub_ty = FuncType::with_finality_and_supertype(
                    &engine,
                    Finality::Final,
                    Some(&super_ty),
                    [ValType::I32],
                    [ValType::I64],
                )?;
                assert!(
                    sub_ty.matches(&super_ty),
                    "{sub_ty:?} should match {super_ty:?}"
                );

                let tag_ty = TagType::new(c.clone());
                let tag1 = Tag::new(&mut store, &tag_ty)?;
                let tag2 = Tag::new(&mut store, &tag_ty)?;
                assert!(FuncType::eq(tag1.ty(&store).ty(), &c));
                assert!(Tag::eq(&tag1, &tag1, &store));
                assert!(!Tag::eq(&tag1, &tag2, &store));

                let func = Func::try_wrap(&mut store, |x: i32| x * 2)?;
                let func_ty = func.ty(&store);
                assert_eq!(func_ty.params().len(), 1);
                assert_eq!(func_ty.results().len(), 1);
                assert!(FuncType::eq(
                    &func_ty,
                    &FuncType::try_new(&engine, [ValType::I32], [ValType::I32])?
                ));
                let _typed = func.typed::<i32, i32>(&store)?;

                // The module's types must survive re-registration intact.
                let module = unsafe { Module::deserialize(&engine, &module_bytes)? };
                let c_ty = module.get_export("c").unwrap().unwrap_func().clone();
                assert_eq!(c_ty.params().len(), 2);
                assert_eq!(c_ty.results().len(), 1);
                let c_ty2 =
                    FuncType::try_new(&engine, [ValType::I32, ValType::I32], [ValType::I64])?;
                assert!(
                    FuncType::eq(&c_ty, &c_ty2),
                    "{c_ty:?} should be equal to {c_ty2:?}"
                );

                let component = unsafe { Component::deserialize(&engine, &component_bytes)? };
                let linker = Linker::<()>::new(&engine);
                let instance = linker.instantiate(&mut store, &component)?;
                let id = instance.get_typed_func::<(i32,), (i32,)>(&mut store, "id")?;
                assert_eq!(id.call(&mut store, (42,))?, (42,));

                // Symbolication reads the global code registry.
                let boom = instance.get_typed_func::<(), ()>(&mut store, "boom")?;
                let err = boom.call(&mut store, ()).unwrap_err();
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
}
