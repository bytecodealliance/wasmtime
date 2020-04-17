use anyhow::Result;
use wasmtime::*;

#[test]
fn wrong_import_numbers() -> Result<()> {
    let store = Store::default();
    let module = Module::new(&store, r#"(module (import "" "" (func)))"#)?;

    assert!(Instance::new(&module, &[]).is_err());
    let func = Func::wrap(&store, || {});
    assert!(Instance::new(&module, &[func.clone().into(), func.into()]).is_err());
    Ok(())
}
