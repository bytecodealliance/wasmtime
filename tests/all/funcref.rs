use super::ref_types_module;
use std::sync::atomic::{AtomicBool, Ordering::SeqCst};
use std::sync::Arc;
use wasmtime::*;

#[test]
fn pass_funcref_in_and_out_of_wasm() -> anyhow::Result<()> {
    let (mut store, module) = ref_types_module(
        false,
        r#"
            (module
                (func (export "func") (param funcref) (result funcref)
                    local.get 0
                )
            )
        "#,
    )?;

    let instance = Instance::new(&mut store, &module, &[])?;
    let func = instance.get_func(&mut store, "func").unwrap();

    // Pass in a non-null funcref.
    {
        let mut results = [Val::I32(0)];
        func.call(
            &mut store,
            &[Val::FuncRef(Some(func.clone()))],
            &mut results,
        )?;

        // Can't compare `Func` for equality, so this is the best we can do here.
        let result_func = results[0].unwrap_funcref().unwrap();
        assert_eq!(func.ty(&store), result_func.ty(&store));
    }

    // Pass in a null funcref.
    {
        let mut results = [Val::I32(0)];
        func.call(&mut store, &[Val::FuncRef(None)], &mut results)?;
        let result_func = results[0].unwrap_funcref();
        assert!(result_func.is_none());
    }

    // Pass in a `funcref` from another instance.
    {
        let other_instance = Instance::new(&mut store, &module, &[])?;
        let other_instance_func = other_instance.get_func(&mut store, "func").unwrap();

        let mut results = [Val::I32(0)];
        func.call(
            &mut store,
            &[Val::FuncRef(Some(other_instance_func.clone()))],
            &mut results,
        )?;
        assert_eq!(results.len(), 1);

        // Can't compare `Func` for equality, so this is the best we can do here.
        let result_func = results[0].unwrap_funcref().unwrap();
        assert_eq!(other_instance_func.ty(&store), result_func.ty(&store));
    }

    // Passing in a `funcref` from another store fails.
    {
        let (mut other_store, other_module) =
            ref_types_module(false, r#"(module (func (export "f")))"#)?;
        let other_store_instance = Instance::new(&mut other_store, &other_module, &[])?;
        let f = other_store_instance
            .get_func(&mut other_store, "f")
            .unwrap();

        assert!(func
            .call(&mut store, &[Val::FuncRef(Some(f))], &mut [Val::I32(0)])
            .is_err());
    }

    Ok(())
}

#[test]
fn receive_null_funcref_from_wasm() -> anyhow::Result<()> {
    let (mut store, module) = ref_types_module(
        false,
        r#"
            (module
                (func (export "get-null") (result funcref)
                    ref.null func
                )
            )
        "#,
    )?;

    let instance = Instance::new(&mut store, &module, &[])?;
    let get_null = instance.get_func(&mut store, "get-null").unwrap();

    let mut results = [Val::I32(0)];
    get_null.call(&mut store, &[], &mut results)?;
    let result_func = results[0].unwrap_funcref();
    assert!(result_func.is_none());

    Ok(())
}

#[test]
fn wrong_store() -> anyhow::Result<()> {
    let dropped = Arc::new(AtomicBool::new(false));
    {
        let mut store1 = Store::<()>::default();
        let mut store2 = Store::<()>::default();

        let set = SetOnDrop(dropped.clone());
        let f1 = Func::wrap(&mut store1, move || drop(&set));
        let f2 = Func::wrap(&mut store2, move || Some(f1.clone()));
        assert!(f2.call(&mut store2, &[], &mut []).is_err());
    }
    assert!(dropped.load(SeqCst));

    return Ok(());

    struct SetOnDrop(Arc<AtomicBool>);

    impl Drop for SetOnDrop {
        fn drop(&mut self) {
            self.0.store(true, SeqCst);
        }
    }
}

#[test]
fn func_new_returns_wrong_store() -> anyhow::Result<()> {
    let dropped = Arc::new(AtomicBool::new(false));
    {
        let mut store1 = Store::<()>::default();
        let mut store2 = Store::<()>::default();

        let set = SetOnDrop(dropped.clone());
        let f1 = Func::wrap(&mut store1, move || drop(&set));
        let f2 = Func::new(
            &mut store2,
            FuncType::new(None, Some(ValType::FuncRef)),
            move |_, _, results| {
                results[0] = f1.clone().into();
                Ok(())
            },
        );
        assert!(f2.call(&mut store2, &[], &mut [Val::I32(0)]).is_err());
    }
    assert!(dropped.load(SeqCst));

    return Ok(());

    struct SetOnDrop(Arc<AtomicBool>);

    impl Drop for SetOnDrop {
        fn drop(&mut self) {
            self.0.store(true, SeqCst);
        }
    }
}
