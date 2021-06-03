use anyhow::Result;
use wasmtime::*;

fn engine() -> Engine {
    let mut config = Config::new();
    config.wasm_module_linking(true);
    Engine::new(&config).unwrap()
}

#[test]
fn compile() -> Result<()> {
    let engine = engine();
    Module::new(&engine, "(module (module))")?;
    Module::new(&engine, "(module (module) (module))")?;
    Module::new(&engine, "(module (module (module)))")?;
    Module::new(
        &engine,
        "
            (module
                (func)
                (module (func))
                (module (func))
            )
        ",
    )?;
    let m = Module::new(
        &engine,
        "
            (module
                (global i32 (i32.const 0))
                (func)
                (module (memory 1) (func))
                (module (memory 2) (func))
                (module (table 2 funcref) (func))
                (module (global i64 (i64.const 0)) (func))
            )
        ",
    )?;
    assert_eq!(m.imports().len(), 0);
    assert_eq!(m.exports().len(), 0);
    let bytes = m.serialize()?;
    unsafe {
        Module::deserialize(&engine, &bytes)?;
    }
    assert_eq!(m.imports().len(), 0);
    assert_eq!(m.exports().len(), 0);
    Ok(())
}

#[test]
fn types() -> Result<()> {
    let engine = engine();
    Module::new(&engine, "(module (type (module)))")?;
    Module::new(&engine, "(module (type (instance)))")?;
    Ok(())
}

#[test]
fn imports_exports() -> Result<()> {
    let engine = engine();

    // empty module type
    let module = Module::new(&engine, "(module (module (export \"\")))")?;
    let mut e = module.exports();
    assert_eq!(e.len(), 1);
    let export = e.next().unwrap();
    assert_eq!(export.name(), "");
    let module_ty = match export.ty() {
        ExternType::Module(m) => m,
        _ => panic!("unexpected type"),
    };
    assert_eq!(module_ty.imports().len(), 0);
    assert_eq!(module_ty.exports().len(), 0);

    // empty instance type
    let module = Module::new(
        &engine,
        "
            (module
                (module)
                (instance (export \"\") (instantiate 0)))
        ",
    )?;
    let mut e = module.exports();
    assert_eq!(e.len(), 1);
    let export = e.next().unwrap();
    assert_eq!(export.name(), "");
    let instance_ty = match export.ty() {
        ExternType::Instance(i) => i,
        _ => panic!("unexpected type"),
    };
    assert_eq!(instance_ty.exports().len(), 0);

    // full module type
    let module = Module::new(
        &engine,
        "
            (module
                (import \"\" \"a\" (module
                    (import \"a\" (func))
                    (export \"\" (global i32))
                ))
            )
        ",
    )?;
    let mut i = module.imports();
    assert_eq!(i.len(), 1);
    let import = i.next().unwrap();
    assert_eq!(import.module(), "");
    assert_eq!(import.name(), None);
    let instance_ty = match import.ty() {
        ExternType::Instance(t) => t,
        _ => panic!("unexpected type"),
    };
    assert_eq!(instance_ty.exports().len(), 1);
    let module_ty = match instance_ty.exports().next().unwrap().ty() {
        ExternType::Module(m) => m,
        _ => panic!("unexpected type"),
    };
    assert_eq!(module_ty.imports().len(), 1);
    assert_eq!(module_ty.exports().len(), 1);
    let import = module_ty.imports().next().unwrap();
    assert_eq!(import.module(), "a");
    assert_eq!(import.name(), None);
    match import.ty() {
        ExternType::Func(f) => {
            assert_eq!(f.results().len(), 0);
            assert_eq!(f.params().len(), 0);
        }
        _ => panic!("unexpected type"),
    }
    let export = module_ty.exports().next().unwrap();
    assert_eq!(export.name(), "");
    match export.ty() {
        ExternType::Global(g) => {
            assert_eq!(*g.content(), ValType::I32);
            assert_eq!(g.mutability(), Mutability::Const);
        }
        _ => panic!("unexpected type"),
    }

    // full instance type
    let module = Module::new(
        &engine,
        "
            (module
                (import \"\" \"b\" (instance
                    (export \"m\" (memory 1))
                    (export \"t\" (table 1 funcref))
                ))
            )
        ",
    )?;
    let mut i = module.imports();
    assert_eq!(i.len(), 1);
    let import = i.next().unwrap();
    assert_eq!(import.module(), "");
    assert_eq!(import.name(), None);
    let instance_ty = match import.ty() {
        ExternType::Instance(t) => t,
        _ => panic!("unexpected type"),
    };
    assert_eq!(instance_ty.exports().len(), 1);
    let instance_ty = match instance_ty.exports().next().unwrap().ty() {
        ExternType::Instance(m) => m,
        _ => panic!("unexpected type"),
    };
    assert_eq!(instance_ty.exports().len(), 2);
    let mem_export = instance_ty.exports().nth(0).unwrap();
    assert_eq!(mem_export.name(), "m");
    match mem_export.ty() {
        ExternType::Memory(m) => {
            assert_eq!(m.limits().min(), 1);
            assert_eq!(m.limits().max(), None);
        }
        _ => panic!("unexpected type"),
    }
    let table_export = instance_ty.exports().nth(1).unwrap();
    assert_eq!(table_export.name(), "t");
    match table_export.ty() {
        ExternType::Table(t) => {
            assert_eq!(t.limits().min(), 1);
            assert_eq!(t.limits().max(), None);
            assert_eq!(*t.element(), ValType::FuncRef);
        }
        _ => panic!("unexpected type"),
    }
    Ok(())
}

#[test]
fn limit_instances() -> Result<()> {
    let mut config = Config::new();
    config.wasm_module_linking(true);
    let engine = Engine::new(&config)?;
    let module = Module::new(
        &engine,
        r#"
            (module $PARENT
              (module $m0)
              (module $m1
                (instance (instantiate (module outer $PARENT $m0)))
                (instance (instantiate (module outer $PARENT $m0))))
              (module $m2
                (instance (instantiate (module outer $PARENT $m1)))
                (instance (instantiate (module outer $PARENT $m1))))
              (module $m3
                (instance (instantiate (module outer $PARENT $m2)))
                (instance (instantiate (module outer $PARENT $m2))))
              (module $m4
                (instance (instantiate (module outer $PARENT $m3)))
                (instance (instantiate (module outer $PARENT $m3))))
              (module $m5
                (instance (instantiate (module outer $PARENT $m4)))
                (instance (instantiate (module outer $PARENT $m4))))
              (instance (instantiate $m5))
            )
        "#,
    )?;
    let mut store = Store::new(&engine, ());
    store.limiter(StoreLimitsBuilder::new().instances(10).build());
    let err = Instance::new(&mut store, &module, &[]).err().unwrap();
    assert!(
        err.to_string().contains("resource limit exceeded"),
        "bad error: {}",
        err
    );
    Ok(())
}

#[test]
fn limit_memories() -> Result<()> {
    let mut config = Config::new();
    config.wasm_module_linking(true);
    config.wasm_multi_memory(true);
    let engine = Engine::new(&config)?;
    let module = Module::new(
        &engine,
        r#"
            (module
              (module $m0
                (memory 1 1)
                (memory 1 1)
                (memory 1 1)
                (memory 1 1)
                (memory 1 1)
              )

              (instance (instantiate $m0))
              (instance (instantiate $m0))
              (instance (instantiate $m0))
              (instance (instantiate $m0))
            )
        "#,
    )?;
    let mut store = Store::new(&engine, ());
    store.limiter(StoreLimitsBuilder::new().memories(10).build());
    let err = Instance::new(&mut store, &module, &[]).err().unwrap();
    assert!(
        err.to_string().contains("resource limit exceeded"),
        "bad error: {}",
        err
    );
    Ok(())
}

#[test]
fn limit_tables() -> Result<()> {
    let mut config = Config::new();
    config.wasm_module_linking(true);
    let engine = Engine::new(&config)?;
    let module = Module::new(
        &engine,
        r#"
            (module
              (module $m0
                (table 1 1 funcref)
                (table 1 1 funcref)
                (table 1 1 funcref)
                (table 1 1 funcref)
                (table 1 1 funcref)
              )

              (instance (instantiate $m0))
              (instance (instantiate $m0))
              (instance (instantiate $m0))
              (instance (instantiate $m0))
            )
        "#,
    )?;
    let mut store = Store::new(&engine, ());
    store.limiter(StoreLimitsBuilder::new().tables(10).build());
    let err = Instance::new(&mut store, &module, &[]).err().unwrap();
    assert!(
        err.to_string().contains("resource limit exceeded"),
        "bad error: {}",
        err
    );
    Ok(())
}
