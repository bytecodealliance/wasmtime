use super::ref_types_module;
use std::cell::Cell;
use std::rc::Rc;
use wasmtime::*;

#[test]
fn pass_funcref_in_and_out_of_wasm() -> anyhow::Result<()> {
    let (store, module) = ref_types_module(
        r#"
            (module
                (func (export "func") (param funcref) (result funcref)
                    local.get 0
                )
            )
        "#,
    )?;

    let instance = Instance::new(&store, &module, &[])?;
    let func = instance.get_func("func").unwrap();

    // Pass in a non-null funcref.
    {
        let results = func.call(&[Val::FuncRef(Some(func.clone()))])?;
        assert_eq!(results.len(), 1);

        // Can't compare `Func` for equality, so this is the best we can do here.
        let result_func = results[0].unwrap_funcref().unwrap();
        assert_eq!(func.ty(), result_func.ty());
    }

    // Pass in a null funcref.
    {
        let results = func.call(&[Val::FuncRef(None)])?;
        assert_eq!(results.len(), 1);

        let result_func = results[0].unwrap_funcref();
        assert!(result_func.is_none());
    }

    // Pass in a `funcref` from another instance.
    {
        let other_instance = Instance::new(&store, &module, &[])?;
        let other_instance_func = other_instance.get_func("func").unwrap();

        let results = func.call(&[Val::FuncRef(Some(other_instance_func.clone()))])?;
        assert_eq!(results.len(), 1);

        // Can't compare `Func` for equality, so this is the best we can do here.
        let result_func = results[0].unwrap_funcref().unwrap();
        assert_eq!(other_instance_func.ty(), result_func.ty());
    }

    // Passing in a `funcref` from another store fails.
    {
        let (other_store, other_module) = ref_types_module(r#"(module (func (export "f")))"#)?;
        let other_store_instance = Instance::new(&other_store, &other_module, &[])?;
        let f = other_store_instance.get_func("f").unwrap();

        assert!(func.call(&[Val::FuncRef(Some(f))]).is_err());
    }

    Ok(())
}

#[test]
fn receive_null_funcref_from_wasm() -> anyhow::Result<()> {
    let (store, module) = ref_types_module(
        r#"
            (module
                (func (export "get-null") (result funcref)
                    ref.null func
                )
            )
        "#,
    )?;

    let instance = Instance::new(&store, &module, &[])?;
    let get_null = instance.get_func("get-null").unwrap();

    let results = get_null.call(&[])?;
    assert_eq!(results.len(), 1);

    let result_func = results[0].unwrap_funcref();
    assert!(result_func.is_none());

    Ok(())
}

#[test]
fn wrong_store() -> anyhow::Result<()> {
    let dropped = Rc::new(Cell::new(false));
    {
        let store1 = Store::default();
        let store2 = Store::default();

        let set = SetOnDrop(dropped.clone());
        let f1 = Func::wrap(&store1, move || drop(&set));
        let f2 = Func::wrap(&store2, move || Some(f1.clone()));
        assert!(f2.call(&[]).is_err());
    }
    assert!(dropped.get());

    return Ok(());

    struct SetOnDrop(Rc<Cell<bool>>);

    impl Drop for SetOnDrop {
        fn drop(&mut self) {
            self.0.set(true);
        }
    }
}
