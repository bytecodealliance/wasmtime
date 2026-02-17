#![cfg(not(miri))] // not testing unsafe code

use wasmtime::*;

fn async_store() -> Store<()> {
    let engine = Engine::default();
    let mut store = Store::new(&engine, ());
    Func::wrap_async(&mut store, |_, ()| Box::new(async {}));
    return store;
}

struct MyAsyncLimiter;

#[async_trait::async_trait]
impl ResourceLimiterAsync for MyAsyncLimiter {
    async fn memory_growing(
        &mut self,
        _current: usize,
        _desired: usize,
        _maximum: Option<usize>,
    ) -> Result<bool> {
        Ok(true)
    }

    async fn table_growing(
        &mut self,
        _current: usize,
        _desired: usize,
        _maximum: Option<usize>,
    ) -> Result<bool> {
        Ok(true)
    }
}

fn async_limiter_store() -> Store<MyAsyncLimiter> {
    let engine = Engine::default();
    let mut store = Store::new(&engine, MyAsyncLimiter);
    store.limiter_async(|x| x);
    return store;
}

fn assert_requires_async<T>(store: &mut Store<T>) {
    let module = Module::new(store.engine(), "(module)").unwrap();
    assert!(Instance::new(&mut *store, &module, &[]).is_err());
}

#[test]
fn require_async_after_func_wrap() {
    let engine = Engine::default();
    let mut store = Store::new(&engine, ());
    Func::wrap_async(&mut store, |_, ()| Box::new(async {}));
    assert_requires_async(&mut store);
}

#[test]
fn require_async_after_func_new() {
    let engine = Engine::default();
    let mut store = Store::new(&engine, ());
    let ty = FuncType::new(store.engine(), [], []);
    Func::new_async(&mut store, ty, |_, _, _| Box::new(async { Ok(()) }));
    assert_requires_async(&mut store);
}

#[tokio::test]
async fn require_async_after_linker_with_func_wrap() -> Result<()> {
    let engine = Engine::default();
    let mut store = Store::new(&engine, ());
    let mut linker = Linker::new(store.engine());
    linker.func_wrap_async("", "", |_, ()| Box::new(async {}))?;
    let module = Module::new(store.engine(), r#"(module (import "" "" (func)))"#)?;
    linker.instantiate_async(&mut store, &module).await?;
    assert_requires_async(&mut store);
    Ok(())
}

#[tokio::test]
async fn require_async_after_linker_with_func_new() -> Result<()> {
    let engine = Engine::default();
    let mut store = Store::new(&engine, ());
    let mut linker = Linker::new(store.engine());
    let ty = FuncType::new(store.engine(), [], []);
    linker.func_new_async("", "", ty, |_, _, _| Box::new(async { Ok(()) }))?;
    let module = Module::new(store.engine(), r#"(module (import "" "" (func)))"#)?;
    linker.instantiate_async(&mut store, &module).await?;
    assert_requires_async(&mut store);
    Ok(())
}

#[test]
fn require_async_after_epochs() -> Result<()> {
    let mut config = Config::new();
    config.epoch_interruption(true);
    let engine = Engine::new(&config)?;
    let mut store = Store::new(&engine, ());
    store.epoch_deadline_async_yield_and_update(1);
    assert_requires_async(&mut store);
    Ok(())
}

#[test]
fn require_async_after_fuel() -> Result<()> {
    let mut config = Config::new();
    config.consume_fuel(true);
    let engine = Engine::new(&config)?;
    let mut store = Store::new(&engine, ());
    store.fuel_async_yield_interval(Some(1))?;
    assert_requires_async(&mut store);
    Ok(())
}

#[test]
fn require_async_after_async_limiter() -> Result<()> {
    let mut store = async_limiter_store();
    assert_requires_async(&mut store);
    Ok(())
}

#[tokio::test]
async fn require_async_with_linker_func_wrap() -> Result<()> {
    let engine = Engine::default();
    let mut store = Store::new(&engine, ());
    let mut linker = Linker::new(store.engine());
    linker.func_wrap_async("", "", |_, ()| Box::new(async {}))?;
    let module = Module::new(store.engine(), r#"(module (import "" "" (func)))"#)?;
    assert!(linker.instantiate(&mut store, &module).is_err());
    linker.instantiate_async(&mut store, &module).await?;
    Ok(())
}

#[test]
fn require_async_with_debug_handler() -> Result<()> {
    let mut config = Config::new();
    config.guest_debug(true);
    let engine = Engine::new(&config)?;
    let mut store = Store::new(&engine, ());
    store.set_debug_handler(MyDebugHandler);
    assert_requires_async(&mut store);
    return Ok(());

    #[derive(Clone)]
    struct MyDebugHandler;

    impl DebugHandler for MyDebugHandler {
        type Data = ();

        async fn handle(&self, _: StoreContextMut<'_, ()>, _: DebugEvent<'_>) {}
    }
}

struct MyAsyncCallHook;

#[async_trait::async_trait]
impl CallHookHandler<()> for MyAsyncCallHook {
    async fn handle_call_event(&self, _: StoreContextMut<'_, ()>, _: CallHook) -> Result<()> {
        Ok(())
    }
}

#[test]
fn require_async_with_async_call_hook() -> Result<()> {
    let engine = Engine::default();
    let mut store = Store::new(&engine, ());
    store.call_hook_async(MyAsyncCallHook);
    assert_requires_async(&mut store);
    Ok(())
}

#[tokio::test]
async fn async_disallows_instance_new() -> Result<()> {
    let mut store = async_store();
    let module = Module::new(store.engine(), "(module)")?;
    assert!(Instance::new(&mut store, &module, &[]).is_err());
    Instance::new_async(&mut store, &module, &[]).await?;
    Ok(())
}

#[tokio::test]
async fn async_disallows_linker_instantiate() -> Result<()> {
    let mut store = async_store();
    let module = Module::new(store.engine(), "(module)")?;
    let linker = Linker::new(store.engine());
    assert!(linker.instantiate(&mut store, &module).is_err());
    linker.instantiate_async(&mut store, &module).await?;
    Ok(())
}

#[tokio::test]
async fn async_disallows_func_call() -> Result<()> {
    let mut store = async_store();
    let func = Func::wrap(&mut store, || {});
    assert!(func.call(&mut store, &[], &mut []).is_err());
    func.call_async(&mut store, &[], &mut []).await?;
    Ok(())
}

#[tokio::test]
async fn async_disallows_typed_func_call() -> Result<()> {
    let mut store = async_store();
    let func = Func::wrap(&mut store, || {});
    let func = func.typed::<(), ()>(&mut store)?;
    assert!(func.call(&mut store, ()).is_err());
    func.call_async(&mut store, ()).await?;
    Ok(())
}

#[tokio::test]
async fn async_disallows_gc() -> Result<()> {
    let mut store = async_limiter_store();
    assert!(store.gc(None).is_err());
    store.gc_async(None).await;
    Ok(())
}

#[tokio::test]
async fn async_disallows_array_ref_new() -> Result<()> {
    let mut store = async_limiter_store();
    let ty = ArrayType::new(
        store.engine(),
        FieldType::new(Mutability::Var, StorageType::I8),
    );
    let pre = ArrayRefPre::new(&mut store, ty);
    assert!(ArrayRef::new(&mut store, &pre, &Val::I32(0), 10).is_err());
    ArrayRef::new_async(&mut store, &pre, &Val::I32(0), 10).await?;
    Ok(())
}

#[tokio::test]
async fn async_disallows_array_ref_new_fixed() -> Result<()> {
    let mut store = async_limiter_store();
    let ty = ArrayType::new(
        store.engine(),
        FieldType::new(Mutability::Var, StorageType::I8),
    );
    let pre = ArrayRefPre::new(&mut store, ty);
    assert!(ArrayRef::new_fixed(&mut store, &pre, &[Val::I32(0)]).is_err());
    ArrayRef::new_fixed_async(&mut store, &pre, &[Val::I32(0)]).await?;
    Ok(())
}

#[tokio::test]
async fn async_disallows_exnref_new() -> Result<()> {
    let mut store = async_limiter_store();
    let ty = ExnType::new(store.engine(), [])?;
    let pre = ExnRefPre::new(&mut store, ty);
    let fty = FuncType::new(store.engine(), [], []);
    let tag = Tag::new(&mut store, &TagType::new(fty))?;
    assert!(ExnRef::new(&mut store, &pre, &tag, &[]).is_err());
    ExnRef::new_async(&mut store, &pre, &tag, &[]).await?;
    Ok(())
}

#[tokio::test]
async fn async_disallows_externref_new() -> Result<()> {
    let mut store = async_limiter_store();
    assert!(ExternRef::new(&mut store, 1).is_err());
    ExternRef::new_async(&mut store, 1).await?;
    Ok(())
}

#[tokio::test]
async fn async_disallows_structref_new() -> Result<()> {
    let mut store = async_limiter_store();
    let ty = StructType::new(
        store.engine(),
        [FieldType::new(Mutability::Var, StorageType::I8)],
    )?;
    let pre = StructRefPre::new(&mut store, ty);
    assert!(StructRef::new(&mut store, &pre, &[Val::I32(0)]).is_err());
    StructRef::new_async(&mut store, &pre, &[Val::I32(0)]).await?;
    Ok(())
}

#[test]
fn epoch_yield_disallowed_without_async() -> Result<()> {
    let mut config = Config::new();
    config.epoch_interruption(true);
    let engine = Engine::new(&config)?;
    let mut store = Store::new(&engine, ());

    let module = Module::new(&engine, r#"(module (func (export "") (loop br 0)))"#)?;
    let instance = Instance::new(&mut store, &module, &[])?;
    let func = instance.get_typed_func::<(), ()>(&mut store, "")?;

    store.epoch_deadline_callback(|_store| Ok(UpdateDeadline::Yield(1)));
    assert!(func.call(&mut store, ()).is_err());

    store.epoch_deadline_callback(|_store| Ok(UpdateDeadline::YieldCustom(1, Box::pin(async {}))));
    assert!(func.call(&mut store, ()).is_err());

    store.epoch_deadline_trap();
    let err = func.call(&mut store, ()).unwrap_err().downcast::<Trap>()?;
    assert_eq!(err, Trap::Interrupt);
    Ok(())
}

#[test]
fn start_sync_then_configure_async_then_do_async() -> Result<()> {
    let mut config = Config::new();
    config.consume_fuel(true);
    let engine = Engine::new(&config)?;
    let mut store = Store::new(&engine, MyAsyncLimiter);

    let module = Module::new(
        &engine,
        r#"
            (module
                (import "" "" (func $host))
                (memory 1)
                (func (export "")
                    call $host
                    (loop br 0)
                )
            )
        "#,
    )?;

    let mut linker = Linker::new(&engine);
    linker.func_wrap("", "", |mut store: Caller<'_, MyAsyncLimiter>| {
        store.as_context_mut().fuel_async_yield_interval(Some(1))
    })?;
    store.set_fuel(100)?;
    let instance = linker.instantiate(&mut store, &module)?;
    let func = instance.get_typed_func::<(), ()>(&mut store, "")?;
    let err = func.call(&mut store, ()).unwrap_err();
    assert!(
        format!("{err:?}").contains("configured to do async things"),
        "bad error {err:?}",
    );
    Ok(())
}

#[tokio::test]
async fn async_limiter_disallows_table_new() -> Result<()> {
    let mut store = async_limiter_store();
    let ty = TableType::new(RefType::FUNCREF, 1, None);
    assert!(Table::new(&mut store, ty.clone(), Ref::Func(None)).is_err());
    Table::new_async(&mut store, ty, Ref::Func(None)).await?;
    Ok(())
}

#[tokio::test]
async fn async_limiter_disallows_table_grow() -> Result<()> {
    let mut store = async_limiter_store();
    let ty = TableType::new(RefType::FUNCREF, 1, None);
    let table = Table::new_async(&mut store, ty, Ref::Func(None)).await?;
    assert!(table.grow(&mut store, 1, Ref::Func(None)).is_err());
    table.grow_async(&mut store, 1, Ref::Func(None)).await?;
    Ok(())
}

#[tokio::test]
async fn async_limiter_disallows_memory_new() -> Result<()> {
    let mut store = async_limiter_store();
    let ty = MemoryType::new(1, None);
    assert!(Memory::new(&mut store, ty.clone()).is_err());
    Memory::new_async(&mut store, ty).await?;
    Ok(())
}

#[tokio::test]
async fn async_limiter_disallows_memory_grow() -> Result<()> {
    let mut store = async_limiter_store();
    let ty = MemoryType::new(1, None);
    let mem = Memory::new_async(&mut store, ty).await?;
    assert!(mem.grow(&mut store, 1).is_err());
    mem.grow_async(&mut store, 1).await?;
    Ok(())
}
