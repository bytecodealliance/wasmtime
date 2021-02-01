use anyhow::Result;
use wasmtime_wasi::{Wasi, WasiCtx};
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

    let store = Store::default();
    let module = Module::new(store.engine(), WAT).expect("failed to create module");

    let other = Rc::new(RefCell::new(None::<Func>));
    let other2 = Rc::downgrade(&other);

    let callback_func = Func::new(&store, FuncType::new(None, None), move |_, _, _| {
        other2
            .upgrade()
            .unwrap()
            .borrow()
            .as_ref()
            .expect("expected a function ref")
            .call(&[])
            .expect("expected function not to trap");
        Ok(())
    });

    let imports = vec![callback_func.into()];
    let instance =
        Instance::new(&store, &module, imports.as_slice()).expect("failed to instantiate module");

    let run_func = instance
        .get_func("run")
        .expect("expected a run func in the module");

    *other.borrow_mut() = Some(
        instance
            .get_func("other")
            .expect("expected an other func in the module"),
    );

    run_func.call(&[]).expect("expected function not to trap");
}

#[test]
fn test_returns_incorrect_type() -> Result<()> {
    const WAT: &str = r#"
    (module
        (import "env" "evil" (func $evil (result i32)))
        (func (export "run") (result i32)
            (call $evil)
        )
    )
    "#;

    let store = Store::default();
    let module = Module::new(store.engine(), WAT)?;

    let callback_func = Func::new(
        &store,
        FuncType::new(None, Some(ValType::I32)),
        |_, _, results| {
            // Evil! Returns I64 here instead of promised in the signature I32.
            results[0] = Val::I64(228);
            Ok(())
        },
    );

    let imports = vec![callback_func.into()];
    let instance = Instance::new(&store, &module, imports.as_slice())?;

    let run_func = instance
        .get_func("run")
        .expect("expected a run func in the module");

    let trap = run_func
        .call(&[])
        .expect_err("the execution should fail")
        .downcast::<Trap>()?;
    assert!(trap
        .to_string()
        .contains("function attempted to return an incompatible value"));
    Ok(())
}

#[test]
#[cfg_attr(not(feature = "unstable-allow-missing-imports"), ignore)]
fn unstable_allow_missing_imports() -> Result<()> {
    const WAT: &str = r#"
    (module
        (type $t0 (func))
        (import "env" "missing_function" (func $missing_function (type $t0)))
        (func $main (type $t0)
          call $missing_function
          return))
    "#;

    let store = Store::default();
    let mut linker = Linker::new(&store);
    let wasi = Wasi::new(&store, WasiCtx::new(std::env::args())?);
    wasi.add_to_linker(&mut linker)?;
    
    let module = Module::new(store.engine(), WAT)?;
    linker.module("", &module)?;

    Ok(())
}
