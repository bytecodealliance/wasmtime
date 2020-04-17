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

    let store = Store::default();
    let module = Module::new(&store, WAT)?;

    let imports = [
        Func::new(
            &store,
            FuncType::new(Box::new([]), Box::new([ValType::I32])),
            |_, params, results| {
                assert!(params.is_empty());
                assert_eq!(results.len(), 1);
                results[0] = 1i32.into();
                Ok(())
            },
        )
        .into(),
        Func::new(
            &store,
            FuncType::new(Box::new([]), Box::new([ValType::F32])),
            |_, params, results| {
                assert!(params.is_empty());
                assert_eq!(results.len(), 1);
                results[0] = 2.0f32.into();
                Ok(())
            },
        )
        .into(),
    ];
    let instance = Instance::new(&module, &imports)?;

    let func = instance.get_export("foo").unwrap().func().unwrap();
    let results = func.call(&[])?;
    assert_eq!(results.len(), 1);
    match results[0] {
        Val::I32(n) => assert_eq!(n, 3),
        _ => panic!("unexpected type of return"),
    }
    Ok(())
}
