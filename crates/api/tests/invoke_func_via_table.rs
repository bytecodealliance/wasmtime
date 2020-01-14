use anyhow::{Context as _, Result};
use wasmtime::*;

#[test]
fn test_invoke_func_via_table() -> Result<()> {
    let store = Store::default();

    let binary = wat::parse_str(
        r#"
          (module
            (func $f (result i64) (i64.const 42))

            (table (export "table") 1 1 anyfunc)
            (elem (i32.const 0) $f)
          )
        "#,
    )?;
    let module = Module::new(&store, &binary).context("> Error compiling module!")?;
    let instance = Instance::new(&module, &[]).context("> Error instantiating module!")?;

    let f = instance
        .find_export_by_name("table")
        .unwrap()
        .table()
        .unwrap()
        .get(0)
        .funcref()
        .unwrap()
        .clone();
    let result = f.call(&[]).unwrap();
    assert_eq!(result[0].unwrap_i64(), 42);
    Ok(())
}
