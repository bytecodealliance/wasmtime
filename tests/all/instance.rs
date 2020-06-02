use anyhow::Result;
use wasmtime::*;

#[test]
fn wrong_import_numbers() -> Result<()> {
    let store = Store::default();
    let module = Module::new(store.engine(), r#"(module (import "" "" (func)))"#)?;

    assert!(Instance::new(&store, &module, &[]).is_err());
    let func = Func::wrap(&store, || {});
    assert!(Instance::new(&store, &module, &[func.clone().into(), func.into()]).is_err());
    Ok(())
}
