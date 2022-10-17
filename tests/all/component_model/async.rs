use anyhow::Result;
use wasmtime::component::*;
use wasmtime::{Store, StoreContextMut, Trap, TrapCode};

#[tokio::test]
async fn thunks() -> Result<()> {
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
    // TODO fold instantiate_async into Linker as well
    let instance = Linker::new(&engine)
        .instantiate_pre(&component)?
        .instantiate_async(&mut store)
        .await?;

    let thunk = instance.get_typed_func::<(), (), _>(&mut store, "thunk")?;

    thunk.call_async(&mut store, ()).await?;
    // TODO post_return_async required as well
    thunk.post_return(&mut store)?;

    let err = instance
        .get_typed_func::<(), (), _>(&mut store, "thunk-trap")?
        .call_async(&mut store, ())
        .await
        .unwrap_err();
    assert!(err.downcast::<Trap>()?.trap_code() == Some(TrapCode::UnreachableCodeReached));

    Ok(())
}

#[tokio::test]
async fn thunks_2() -> Result<()> {
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
    imports.func_wrap_async("i", |_: StoreContextMut<()>, _: ()| Box::new(async { () }))?;

    // TODO fold instantiate_async into Linker as well
    let instance = linker
        .instantiate_pre(&component)?
        .instantiate_async(&mut store)
        .await?;

    let thunk = instance.get_typed_func::<(), (), _>(&mut store, "thunk")?;

    thunk.call_async(&mut store, ()).await?;
    thunk.post_return(&mut store)?;

    Ok(())
}
