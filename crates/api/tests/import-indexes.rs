use std::rc::Rc;
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

    struct Ret1;

    impl Callable for Ret1 {
        fn call(&self, params: &[Val], results: &mut [Val]) -> Result<(), Trap> {
            assert!(params.is_empty());
            assert_eq!(results.len(), 1);
            results[0] = 1i32.into();
            Ok(())
        }
    }

    struct Ret2;

    impl Callable for Ret2 {
        fn call(&self, params: &[Val], results: &mut [Val]) -> Result<(), Trap> {
            assert!(params.is_empty());
            assert_eq!(results.len(), 1);
            results[0] = 2.0f32.into();
            Ok(())
        }
    }

    let store = Store::default();
    let wasm = wat::parse_str(WAT)?;
    let module = Module::new(&store, &wasm)?;

    let imports = [
        Func::new(
            &store,
            FuncType::new(Box::new([]), Box::new([ValType::I32])),
            Rc::new(Ret1),
        )
        .into(),
        Func::new(
            &store,
            FuncType::new(Box::new([]), Box::new([ValType::F32])),
            Rc::new(Ret2),
        )
        .into(),
    ];
    let instance = Instance::new(&module, &imports)?;

    let func = instance.find_export_by_name("foo").unwrap().func().unwrap();
    let results = func.call(&[])?;
    assert_eq!(results.len(), 1);
    match results[0] {
        Val::I32(n) => assert_eq!(n, 3),
        _ => panic!("unexpected type of return"),
    }
    Ok(())
}
