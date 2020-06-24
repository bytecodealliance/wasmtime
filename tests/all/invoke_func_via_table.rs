use anyhow::{Context as _, Result};
use wasmtime::*;

#[test]
fn test_invoke_func_via_table() -> Result<()> {
    let store = Store::default();

    let wat = r#"
      (module
        (func $f (result i64) (i64.const 42))

        (table (export "table") 1 1 anyfunc)
        (elem (i32.const 0) $f)
      )
    "#;
    let module = Module::new(store.engine(), wat).context("> Error compiling module!")?;
    let instance = Instance::new(&store, &module, &[]).context("> Error instantiating module!")?;

    let f = instance
        .get_table("table")
        .unwrap()
        .get(0)
        .unwrap()
        .funcref()
        .unwrap()
        .unwrap()
        .clone();
    let result = f.call(&[]).unwrap();
    assert_eq!(result[0].unwrap_i64(), 42);
    Ok(())
}
