use anyhow::Result;
use wasmtime::component::*;
use wasmtime::{Store, StoreContextMut, Trap, TrapCode};

/// This is super::func::thunks, except with an async store.
#[tokio::test]
async fn smoke() -> Result<()> {
    let component = r#"
        (component
            (core module $m
                (func (export "thunk"))
                (func (export "thunk-trap") unreachable)
            )
            (core instance $i (instantiate $m))
            (func (export "thunk")
                (canon lift (core func $i "thunk"))
            )
            (func (export "thunk-trap")
                (canon lift (core func $i "thunk-trap"))
            )
        )
    "#;

    let engine = super::async_engine();
    let component = Component::new(&engine, component)?;
    let mut store = Store::new(&engine, ());
    let instance = Linker::new(&engine)
        .instantiate_async(&mut store, &component)
        .await?;

    let thunk = instance.get_typed_func::<(), (), _>(&mut store, "thunk")?;

    thunk.call_async(&mut store, ()).await?;
    thunk.post_return_async(&mut store).await?;

    let err = instance
        .get_typed_func::<(), (), _>(&mut store, "thunk-trap")?
        .call_async(&mut store, ())
        .await
        .unwrap_err();
    assert!(err.downcast::<Trap>()?.trap_code() == Some(TrapCode::UnreachableCodeReached));

    Ok(())
}

/// Handle an import function, created using component::Linker::func_wrap_async.
#[tokio::test]
async fn smoke_func_wrap() -> Result<()> {
    let component = r#"
        (component
            (type $f (func))
            (type $imports_type
              (instance
                (alias outer 1 0 (type $f))
                (export "i" (func $f))
              )
            )
            (import "imports" (instance $imports (type $imports_type)))

            (core module $m
                (import "imports" "i" (func $i))
                (func (export "thunk") call $i)
            )

            (alias export 0 "i" (func $f))
            (core func $f_lowered (canon lower (func 0)))
            (core instance $imports_core (export "i" (func $f_lowered)))
            (core instance $i (instantiate $m
                (with "imports" (instance $imports_core))))
            (func (export "thunk")
                (canon lift (core func $i "thunk"))
            )
        )
    "#;

    let engine = super::async_engine();
    let component = Component::new(&engine, component)?;
    let mut store = Store::new(&engine, ());
    let mut linker = Linker::new(&engine);
    let mut imports = linker.instance("imports")?;
    imports.func_wrap_async("i", |_: StoreContextMut<()>, _: ()| {
        Box::new(async { Ok(()) })
    })?;

    let instance = linker.instantiate_async(&mut store, &component).await?;

    let thunk = instance.get_typed_func::<(), (), _>(&mut store, "thunk")?;

    thunk.call_async(&mut store, ()).await?;
    thunk.post_return_async(&mut store).await?;

    Ok(())
}
