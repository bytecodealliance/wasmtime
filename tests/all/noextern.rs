use std::sync::atomic::{AtomicU32, Ordering};
use wasmtime::*;

#[test]
fn func_wrap_no_extern() -> Result<()> {
    let mut config = Config::default();
    config.wasm_function_references(true);
    config.wasm_gc(true);

    let engine = Engine::new(&config)?;

    let module = Module::new(
        &engine,
        r#"
            (module
                (import "" "" (func (param nullexternref) (result nullexternref)))
                (func $main
                    ref.null noextern
                    call 0
                    drop
                )
                (start $main)
            )
        "#,
    )?;

    let mut store = Store::new(&engine, ());

    static HITS: AtomicU32 = AtomicU32::new(0);

    let f = Func::wrap(&mut store, |x: Option<NoExtern>| -> Option<NoExtern> {
        assert!(x.is_none());
        HITS.fetch_add(1, Ordering::SeqCst);
        x
    });

    let _ = Instance::new(&mut store, &module, &[f.into()])?;
    assert_eq!(HITS.load(Ordering::SeqCst), 1);

    Ok(())
}

#[test]
fn func_typed_no_extern() -> Result<()> {
    let mut config = Config::default();
    config.wasm_function_references(true);
    config.wasm_gc(true);

    let engine = Engine::new(&config)?;

    let module = Module::new(
        &engine,
        r#"
            (module
                (func (export "f") (param nullexternref) (result nullexternref)
                    local.get 0
                )
            )
        "#,
    )?;

    let mut store = Store::new(&engine, ());
    let instance = Instance::new(&mut store, &module, &[])?;
    let f = instance.get_typed_func::<Option<NoExtern>, Option<NoExtern>>(&mut store, "f")?;
    let result = f.call(&mut store, None)?;
    assert!(result.is_none());

    Ok(())
}
