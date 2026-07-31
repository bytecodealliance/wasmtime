//! Tests that objects belonging to one `Engine` cannot be admitted into a
//! `Store` that belongs to a different `Engine`.
//!
//! Type indices (`VMSharedTypeIndex`) are unique only within a single
//! `Engine`. Two engines will happily hand out the same index for two
//! completely unrelated types. When an object from one engine reaches a store
//! from another, those numerically-equal-but-semantically-different indices
//! collide:
//!
//! * A host function's `wasm_call` field gets patched with a Wasm-to-array
//!   trampoline compiled for a different signature, so the trampoline reads
//!   and writes the wrong number of `ValRaw` slots in a stack buffer.
//!
//! * The garbage collector traces a Wasm object using a foreign type's
//!   layout.
//!
//! Every API that lets an object into a store must therefore check that the
//! object comes from the store's engine.

use crate::ErrorExt;
use std::sync::{Arc, Mutex};
use wasmtime::component::{Component, Linker as ComponentLinker};
use wasmtime::*;

/// The value that [`observe_i64_in_host`] sends from Wasm to the host.
const SENTINEL: i64 = 0x1122334455667788;

/// A module whose only function type is `(func (param f64))`.
///
/// In a freshly created engine that type gets the same `VMSharedTypeIndex` as
/// the `(func (param i64))` type used by [`observe_i64_in_host`], so if this
/// module is registered with a store belonging to a different engine then its
/// trampoline is what the host function ends up calling.
const COLLIDING_MODULE: &str = r#"(module (func (export "f") (param f64)))"#;

/// [`COLLIDING_MODULE`] wrapped up in a component.
///
/// The core module is deliberately left uninstantiated so that instantiating
/// this component does not itself instantiate any core module.
const COLLIDING_COMPONENT: &str = r#"
    (component
        (core module (func (export "f") (param f64)))
    )
"#;

/// Two separate engines that assign the same type indices to different types.
fn engine_pair() -> (Engine, Engine) {
    (Engine::default(), Engine::default())
}

/// Same as [`engine_pair`], but with a customized configuration.
fn engine_pair_with(f: impl Fn(&mut Config)) -> Result<(Engine, Engine)> {
    let mut config = Config::new();
    f(&mut config);
    Ok((Engine::new(&config)?, Engine::new(&config)?))
}

/// A module that passes [`SENTINEL`] to an imported host function.
const OBSERVER_MODULE: &str = r#"
    (module
        (import "" "" (func $host (param i64)))
        (func (export "go")
            i64.const 0x1122334455667788
            call $host))
"#;

/// Calls a host function that takes an `i64` and returns the value that the
/// host actually observed, which should always be [`SENTINEL`].
///
/// This is the canary for cross-engine corruption: if a foreign module has
/// been registered with `store`, then filling in this host function's
/// `VMFuncRef::wasm_call` finds the foreign engine's `(param f64)` trampoline
/// and the host sees a garbage value instead.
fn observe_i64_in_host(store: &mut Store<()>) -> Result<i64> {
    let observed = Arc::new(Mutex::new(0));
    let host = Func::wrap(&mut *store, {
        let observed = Arc::clone(&observed);
        move |x: i64| *observed.lock().unwrap() = x
    });
    let engine = store.engine().clone();
    let module = Module::new(&engine, OBSERVER_MODULE)?;
    let instance = Instance::new(&mut *store, &module, &[host.into()])?;
    instance
        .get_typed_func::<(), ()>(&mut *store, "go")?
        .call(&mut *store, ())?;
    let observed = *observed.lock().unwrap();
    Ok(observed)
}

/// Same as [`observe_i64_in_host`], but for stores that have async support
/// enabled, where the synchronous entry points may not be used.
async fn observe_i64_in_host_async(store: &mut Store<()>) -> Result<i64> {
    let observed = Arc::new(Mutex::new(0));
    let host = Func::wrap(&mut *store, {
        let observed = Arc::clone(&observed);
        move |x: i64| *observed.lock().unwrap() = x
    });
    let engine = store.engine().clone();
    let module = Module::new(&engine, OBSERVER_MODULE)?;
    let instance = Instance::new_async(&mut *store, &module, &[host.into()]).await?;
    instance
        .get_typed_func::<(), ()>(&mut *store, "go")?
        .call_async(&mut *store, ())
        .await?;
    let observed = *observed.lock().unwrap();
    Ok(observed)
}

/// Asserts that `store` is still usable and that nothing in it confuses one
/// engine's type indices for another's.
fn assert_store_is_uncorrupted(store: &mut Store<()>) -> Result<()> {
    assert_eq!(observe_i64_in_host(store)?, SENTINEL);
    Ok(())
}

/// Async variant of [`assert_store_is_uncorrupted`].
async fn assert_store_is_uncorrupted_async(store: &mut Store<()>) -> Result<()> {
    assert_eq!(observe_i64_in_host_async(store).await?, SENTINEL);
    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn instance_new_rejects_foreign_module() -> Result<()> {
    let (a, b) = engine_pair();
    let mut store = Store::new(&a, ());
    let foreign = Module::new(&b, COLLIDING_MODULE)?;

    Instance::new(&mut store, &foreign, &[])
        .unwrap_err()
        .assert_contains("cross-`Engine`");

    assert_store_is_uncorrupted(&mut store)
}

/// A foreign module's imports must be rejected as cross-engine rather than
/// type checked, since type checking would compare this engine's indices
/// against the foreign engine's.
#[test]
#[cfg_attr(miri, ignore)]
fn instance_new_rejects_foreign_module_with_imports() -> Result<()> {
    let (a, b) = engine_pair();
    let mut store = Store::new(&a, ());

    // Register an unrelated type first so that the host function below does not
    // land on the same index in `a` that the module's imported type lands on in
    // `b`.
    let _ = Func::wrap(&mut store, |_: i32| {});
    let host = Func::wrap(&mut store, |_: i64| {});

    let foreign = Module::new(&b, r#"(module (import "" "" (func (param i64))))"#)?;

    Instance::new(&mut store, &foreign, &[host.into()])
        .unwrap_err()
        .assert_contains("cross-`Engine`");

    assert_store_is_uncorrupted(&mut store)
}

/// A linker and the module it instantiates can belong to the same engine while
/// an individual item defined in that linker came from a store belonging to a
/// different one, so each item has to be checked on its own.
#[test]
#[cfg_attr(miri, ignore)]
fn instantiate_pre_rejects_foreign_definition() -> Result<()> {
    let (a, b) = engine_pair();

    // Register an unrelated type first so that the function below does not land
    // on the same index in `b` that the module's imported type lands on in `a`.
    let mut foreign_store = Store::new(&b, ());
    let _ = Func::wrap(&mut foreign_store, |_: i32| {});
    let foreign = Func::wrap(&mut foreign_store, |_: f64| {});

    let mut linker = Linker::<()>::new(&a);
    linker.define(&foreign_store, "", "", foreign)?;

    let module = Module::new(&a, r#"(module (import "" "" (func (param i64))))"#)?;

    linker
        .instantiate_pre(&module)
        .err()
        .expect("should reject an item defined from a different engine's store")
        .assert_contains("cross-`Engine`");

    let mut store = Store::new(&a, ());
    assert_store_is_uncorrupted(&mut store)
}

#[test]
#[cfg_attr(miri, ignore)]
fn instance_pre_instantiate_rejects_foreign_store() -> Result<()> {
    let (a, b) = engine_pair();
    let mut store = Store::new(&a, ());
    let foreign = Module::new(&b, COLLIDING_MODULE)?;
    let pre = Linker::<()>::new(&b).instantiate_pre(&foreign)?;

    pre.instantiate(&mut store)
        .unwrap_err()
        .assert_contains("cross-`Engine`");

    assert_store_is_uncorrupted(&mut store)
}

#[test]
#[cfg_attr(miri, ignore)]
fn linker_instantiate_pre_rejects_foreign_module() -> Result<()> {
    let (a, b) = engine_pair();
    let foreign = Module::new(&b, COLLIDING_MODULE)?;

    Linker::<()>::new(&a)
        .instantiate_pre(&foreign)
        .err()
        .expect("should reject a module from a different engine")
        .assert_contains("cross-`Engine`");

    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn component_linker_instantiate_pre_rejects_foreign_component() -> Result<()> {
    let (a, b) = engine_pair();
    let foreign = Component::new(&b, COLLIDING_COMPONENT)?;

    ComponentLinker::<()>::new(&a)
        .instantiate_pre(&foreign)
        .err()
        .expect("should reject a component from a different engine")
        .assert_contains("cross-`Engine`");

    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn component_instance_pre_instantiate_rejects_foreign_store() -> Result<()> {
    let (a, b) = engine_pair();
    let mut store = Store::new(&a, ());
    let foreign = Component::new(&b, COLLIDING_COMPONENT)?;
    let pre = ComponentLinker::<()>::new(&b).instantiate_pre(&foreign)?;

    pre.instantiate(&mut store)
        .unwrap_err()
        .assert_contains("cross-`Engine`");

    assert_store_is_uncorrupted(&mut store)
}

#[test]
#[cfg_attr(miri, ignore)]
fn tag_new_rejects_foreign_type() -> Result<()> {
    let (a, b) = engine_pair_with(|config| {
        config.wasm_exceptions(true);
    })?;
    let mut store = Store::new(&a, ());
    let foreign_ty = TagType::new(FuncType::new(&b, [ValType::I32], []));

    Tag::new(&mut store, &foreign_ty)
        .unwrap_err()
        .assert_contains("wrong engine");

    assert_store_is_uncorrupted(&mut store)
}

#[test]
#[cfg_attr(miri, ignore)]
fn table_new_rejects_foreign_type() -> Result<()> {
    let (a, b) = engine_pair_with(|config| {
        config.wasm_gc(true);
    })?;
    let mut store = Store::new(&a, ());
    let foreign_struct = StructType::new(
        &b,
        [FieldType::new(
            Mutability::Const,
            StorageType::ValType(ValType::I32),
        )],
    )?;
    let foreign_heap_ty = HeapType::ConcreteStruct(foreign_struct);
    let foreign_ty = TableType::new(RefType::new(true, foreign_heap_ty.clone()), 0, None);

    Table::new(&mut store, foreign_ty, Ref::null(&foreign_heap_ty))
        .unwrap_err()
        .assert_contains("wrong engine");

    assert_store_is_uncorrupted(&mut store)
}

/// Same as [`table_new_rejects_foreign_type`], but for the async variant, which
/// reaches the same check by a different path.
#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn table_new_async_rejects_foreign_type() -> Result<()> {
    let (a, b) = engine_pair_with(|config| {
        config.wasm_gc(true);
        config.async_support(true);
    })?;
    let mut store = Store::new(&a, ());
    let foreign_struct = StructType::new(
        &b,
        [FieldType::new(
            Mutability::Const,
            StorageType::ValType(ValType::I32),
        )],
    )?;
    let foreign_heap_ty = HeapType::ConcreteStruct(foreign_struct);
    let foreign_ty = TableType::new(RefType::new(true, foreign_heap_ty.clone()), 0, None);

    Table::new_async(&mut store, foreign_ty, Ref::null(&foreign_heap_ty))
        .await
        .unwrap_err()
        .assert_contains("wrong engine");

    assert_store_is_uncorrupted_async(&mut store).await
}

#[test]
#[cfg_attr(miri, ignore)]
fn global_new_rejects_foreign_type() -> Result<()> {
    let (a, b) = engine_pair_with(|config| {
        config.wasm_gc(true);
    })?;
    let mut store = Store::new(&a, ());
    let foreign_struct = StructType::new(
        &b,
        [FieldType::new(
            Mutability::Const,
            StorageType::ValType(ValType::I32),
        )],
    )?;
    let foreign_heap_ty = HeapType::ConcreteStruct(foreign_struct);
    let foreign_ty = GlobalType::new(
        ValType::Ref(RefType::new(true, foreign_heap_ty.clone())),
        Mutability::Const,
    );

    Global::new(&mut store, foreign_ty, Ref::null(&foreign_heap_ty).into())
        .unwrap_err()
        .assert_contains("wrong engine");

    assert_store_is_uncorrupted(&mut store)
}

#[test]
#[should_panic = "wrong engine"]
fn struct_ref_pre_rejects_foreign_type() {
    let (a, b) = engine_pair();
    let mut store = Store::new(&a, ());
    let foreign_ty = StructType::new(
        &b,
        [FieldType::new(
            Mutability::Const,
            StorageType::ValType(ValType::I32),
        )],
    )
    .unwrap();

    let _ = StructRefPre::new(&mut store, foreign_ty);
}

#[test]
#[should_panic = "wrong engine"]
fn array_ref_pre_rejects_foreign_type() {
    let (a, b) = engine_pair();
    let mut store = Store::new(&a, ());
    let foreign_ty = ArrayType::new(
        &b,
        FieldType::new(Mutability::Const, StorageType::ValType(ValType::I32)),
    );

    let _ = ArrayRefPre::new(&mut store, foreign_ty);
}

#[test]
#[should_panic = "wrong engine"]
fn exn_ref_pre_rejects_foreign_type() {
    let (a, b) = engine_pair_with(|config| {
        config.wasm_exceptions(true);
    })
    .unwrap();
    let mut store = Store::new(&a, ());
    let foreign_ty = ExnType::new(&b, [ValType::I32]).unwrap();

    let _ = ExnRefPre::new(&mut store, foreign_ty);
}
