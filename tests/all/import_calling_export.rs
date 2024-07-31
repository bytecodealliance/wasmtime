#![cfg(not(miri))]

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

    let mut store = Store::<Option<Func>>::default();
    let module = Module::new(store.engine(), WAT).expect("failed to create module");

    let func_ty = FuncType::new(store.engine(), None, None);
    let callback_func = Func::new(&mut store, func_ty, move |mut caller, _, _| {
        caller
            .data()
            .unwrap()
            .call(&mut caller, &[], &mut [])
            .expect("expected function not to trap");
        Ok(())
    });

    let imports = vec![callback_func.into()];
    let instance = Instance::new(&mut store, &module, imports.as_slice())
        .expect("failed to instantiate module");

    let run_func = instance
        .get_func(&mut store, "run")
        .expect("expected a run func in the module");

    let other_func = instance
        .get_func(&mut store, "other")
        .expect("expected an other func in the module");
    *store.data_mut() = Some(other_func);

    run_func
        .call(&mut store, &[], &mut [])
        .expect("expected function not to trap");
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

    let mut store = Store::<()>::default();
    let module = Module::new(store.engine(), WAT)?;

    let func_ty = FuncType::new(store.engine(), None, Some(ValType::I32));
    let callback_func = Func::new(&mut store, func_ty, |_, _, results| {
        // Evil! Returns I64 here instead of promised in the signature I32.
        results[0] = Val::I64(228);
        Ok(())
    });

    let imports = vec![callback_func.into()];
    let instance = Instance::new(&mut store, &module, imports.as_slice())?;

    let run_func = instance
        .get_func(&mut store, "run")
        .expect("expected a run func in the module");

    let mut result = [Val::I32(0)];
    let trap = run_func
        .call(&mut store, &[], &mut result)
        .expect_err("the execution should fail");
    assert!(format!("{trap:?}").contains("function attempted to return an incompatible value"));
    Ok(())
}
