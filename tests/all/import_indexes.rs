use wasmtime::*;

#[test]
fn same_import_names_still_distinct() -> anyhow::Result<()> {
    const WAT: &str = r#"
(module
  (import "" "" (func $a (result i32)))
  (import "" "" (func $b (result f32)))
  (func (export "foo") (result i32)
    call $a
    call $b
    i32.trunc_f32_u
    i32.add)
)
    "#;

    let mut store = Store::<()>::default();
    let module = Module::new(store.engine(), WAT)?;

    let imports = [
        Func::new(
            &mut store,
            FuncType::new(None, Some(ValType::I32)),
            |_, params, results| {
                assert!(params.is_empty());
                assert_eq!(results.len(), 1);
                results[0] = 1i32.into();
                Ok(())
            },
        )
        .into(),
        Func::new(
            &mut store,
            FuncType::new(None, Some(ValType::F32)),
            |_, params, results| {
                assert!(params.is_empty());
                assert_eq!(results.len(), 1);
                results[0] = 2.0f32.into();
                Ok(())
            },
        )
        .into(),
    ];
    let instance = Instance::new(&mut store, &module, &imports)?;

    let func = instance.get_typed_func::<(), i32, _>(&mut store, "foo")?;
    let result = func.call(&mut store, ())?;
    assert_eq!(result, 3);
    Ok(())
}
