#![cfg(not(miri))] // not testing unsafe code

use wasmtime::component::{Component, Func, Linker, ResourceAny};
use wasmtime::{Config, Engine, Result, Store};

fn async_store() -> Store<()> {
    let engine = Engine::default();
    let mut store = Store::new(&engine, ());
    wasmtime::Func::wrap_async(&mut store, |_, ()| Box::new(async {}));
    return store;
}

async fn func_in_store(store: &mut Store<()>) -> Result<Func> {
    let component = Component::new(
        store.engine(),
        r#"
            (component
                (core module $a
                    (func (export "hi") )
                )
                (core instance $i (instantiate $a))
                (func (export "hi") (canon lift (core func $i "hi")))
            )
        "#,
    )?;
    let instance = Linker::new(store.engine())
        .instantiate_async(&mut *store, &component)
        .await?;
    let func = instance.get_func(&mut *store, "hi").unwrap();
    Ok(func)
}

fn assert_requires_async<T>(store: &mut Store<T>) {
    let module = wasmtime::Module::new(store.engine(), "(module)").unwrap();
    assert!(wasmtime::Instance::new(&mut *store, &module, &[]).is_err());
}

#[tokio::test]
async fn async_disallows_func_call() -> Result<()> {
    let mut store = async_store();
    let func = func_in_store(&mut store).await?;
    assert!(func.call(&mut store, &[], &mut []).is_err());
    func.call_async(&mut store, &[], &mut []).await?;
    Ok(())
}

#[tokio::test]
async fn async_disallows_typed_func_call() -> Result<()> {
    let mut store = async_store();
    let func = func_in_store(&mut store).await?;
    let func = func.typed::<(), ()>(&store)?;
    assert!(func.call(&mut store, ()).is_err());
    func.call_async(&mut store, ()).await?;
    Ok(())
}

#[tokio::test]
async fn async_disallows_instantiate() -> Result<()> {
    let mut store = async_store();
    let component = Component::new(store.engine(), "(component)")?;
    let linker = Linker::new(store.engine());
    assert!(linker.instantiate(&mut store, &component).is_err());
    linker.instantiate_async(&mut store, &component).await?;
    Ok(())
}

#[tokio::test]
async fn require_async_after_linker_with_func_wrap_async() -> Result<()> {
    let engine = Engine::default();
    let mut store = Store::new(&engine, ());
    let mut linker = Linker::new(store.engine());
    linker
        .root()
        .func_wrap_async("hi", |_, ()| Box::new(async { Ok(()) }))?;
    let module = Component::new(
        store.engine(),
        r#"
            (component
                (import "hi" (func))
                (core func (canon lower (func 0)))
            )
        "#,
    )?;
    linker.instantiate_async(&mut store, &module).await?;
    assert_requires_async(&mut store);
    Ok(())
}

#[tokio::test]
async fn require_async_after_linker_with_func_wrap_concurrent() -> Result<()> {
    let mut config = Config::new();
    config.wasm_component_model_async(true);
    let engine = Engine::new(&config)?;
    let mut store = Store::new(&engine, ());
    let mut linker = Linker::new(store.engine());
    linker
        .root()
        .func_wrap_concurrent("hi", |_, ()| Box::pin(async { Ok(()) }))?;
    let module = Component::new(
        store.engine(),
        r#"
            (component
                (import "hi" (func async))
                (core func (canon lower (func 0)))
            )
        "#,
    )?;
    linker.instantiate_async(&mut store, &module).await?;
    assert_requires_async(&mut store);
    Ok(())
}

#[tokio::test]
async fn require_async_after_linker_with_func_new_async() -> Result<()> {
    let engine = Engine::default();
    let mut store = Store::new(&engine, ());
    let mut linker = Linker::new(store.engine());
    linker
        .root()
        .func_new_async("hi", |_, _, _, _| Box::new(async { Ok(()) }))?;
    let module = Component::new(
        store.engine(),
        r#"
            (component
                (import "hi" (func))
                (core func (canon lower (func 0)))
            )
        "#,
    )?;
    linker.instantiate_async(&mut store, &module).await?;
    assert_requires_async(&mut store);
    Ok(())
}

#[tokio::test]
async fn require_async_after_linker_with_func_new_concurrent() -> Result<()> {
    let mut config = Config::new();
    config.wasm_component_model_async(true);
    let engine = Engine::new(&config)?;
    let mut store = Store::new(&engine, ());
    let mut linker = Linker::new(store.engine());
    linker
        .root()
        .func_new_concurrent("hi", |_, _, _, _| Box::pin(async { Ok(()) }))?;
    let module = Component::new(
        store.engine(),
        r#"
            (component
                (import "hi" (func async))
                (core func (canon lower (func 0)))
            )
        "#,
    )?;
    linker.instantiate_async(&mut store, &module).await?;
    assert_requires_async(&mut store);
    Ok(())
}

#[tokio::test]
async fn async_disallows_resource_any_drop() -> Result<()> {
    let mut store = async_store();
    let component = Component::new(
        store.engine(),
        r#"
            (component
                (type $t' (resource (rep i32)))
                (export $t "t" (type $t'))

                (core func $new (canon resource.new $t))
                (func (export "mk") (param "r" u32) (result (own $t))
                    (canon lift (core func $new))
                )
            )
        "#,
    )?;
    let instance = Linker::new(store.engine())
        .instantiate_async(&mut store, &component)
        .await?;
    let func = instance.get_typed_func::<(u32,), (ResourceAny,)>(&mut store, "mk")?;
    let (resource,) = func.call_async(&mut store, (42,)).await?;

    assert!(resource.resource_drop(&mut store).is_err());
    resource.resource_drop_async(&mut store).await?;

    Ok(())
}
