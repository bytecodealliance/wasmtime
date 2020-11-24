use anyhow::Result;
use wasmtime::*;

fn engine() -> Engine {
    let mut config = Config::new();
    config.wasm_module_linking(true);
    Engine::new(&config)
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
    Module::deserialize(&engine, &bytes)?;
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
    assert_eq!(import.name(), Some("a"));
    let module_ty = match import.ty() {
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
    assert_eq!(import.name(), Some("b"));
    let instance_ty = match import.ty() {
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
