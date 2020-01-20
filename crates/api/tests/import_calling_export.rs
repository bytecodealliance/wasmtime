use std::cell::RefCell;
use std::rc::Rc;
use wasmtime::*;

#[test]
fn test_import_calling_export() {
    const WAT: &str = r#"
    (module
      (type $t0 (func))
      (import "" "imp" (func $.imp (type $t0)))
      (func $run call $.imp)
      (func $other)
      (export "run" (func $run))
      (export "other" (func $other))
    )
    "#;

    struct Callback {
        pub other: RefCell<Option<Func>>,
    }

    impl Callable for Callback {
        fn call(&self, _params: &[Val], _results: &mut [Val]) -> Result<(), Trap> {
            self.other
                .borrow()
                .as_ref()
                .expect("expected a function ref")
                .call(&[])
                .expect("expected function not to trap");
            Ok(())
        }
    }

    let store = Store::default();
    let wasm = wat::parse_str(WAT).unwrap();
    let module = Module::new(&store, &wasm).expect("failed to create module");

    let callback = Rc::new(Callback {
        other: RefCell::new(None),
    });

    let callback_func = Func::new(
        &store,
        FuncType::new(Box::new([]), Box::new([])),
        callback.clone(),
    );

    let imports = vec![callback_func.into()];
    let instance =
        Instance::new(&module, imports.as_slice()).expect("failed to instantiate module");

    let exports = instance.exports();
    assert!(!exports.is_empty());

    let run_func = exports[0]
        .func()
        .expect("expected a run func in the module");

    *callback.other.borrow_mut() = Some(
        exports[1]
            .func()
            .expect("expected an other func in the module")
            .clone(),
    );

    run_func.call(&[]).expect("expected function not to trap");
}
