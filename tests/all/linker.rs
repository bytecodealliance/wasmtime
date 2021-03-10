use anyhow::Result;
use std::cell::Cell;
use std::rc::Rc;
use wasmtime::*;

#[test]
fn link_undefined() -> Result<()> {
    let store = Store::default();
    let linker = Linker::new(&store);
    let module = Module::new(store.engine(), r#"(module (import "" "" (func)))"#)?;
    assert!(linker.instantiate(&module).is_err());
    let module = Module::new(store.engine(), r#"(module (import "" "" (global i32)))"#)?;
    assert!(linker.instantiate(&module).is_err());
    let module = Module::new(store.engine(), r#"(module (import "" "" (memory 1)))"#)?;
    assert!(linker.instantiate(&module).is_err());
    let module = Module::new(
        store.engine(),
        r#"(module (import "" "" (table 1 funcref)))"#,
    )?;
    assert!(linker.instantiate(&module).is_err());
    Ok(())
}

#[test]
fn link_twice_bad() -> Result<()> {
    let store = Store::default();
    let mut linker = Linker::new(&store);

    // functions
    linker.func("", "", || {})?;
    assert!(linker.func("", "", || {}).is_err());
    assert!(linker
        .func("", "", || -> Result<(), Trap> { loop {} })
        .is_err());
    linker.func("", "", |_: i32| {})?;

    // globals
    let ty = GlobalType::new(ValType::I32, Mutability::Const);
    let global = Global::new(&store, ty, Val::I32(0))?;
    linker.define("", "", global.clone())?;
    assert!(linker.define("", "", global.clone()).is_err());

    let ty = GlobalType::new(ValType::I32, Mutability::Var);
    let global = Global::new(&store, ty, Val::I32(0))?;
    linker.define("", "", global.clone())?;
    assert!(linker.define("", "", global.clone()).is_err());

    let ty = GlobalType::new(ValType::I64, Mutability::Const);
    let global = Global::new(&store, ty, Val::I64(0))?;
    linker.define("", "", global.clone())?;
    assert!(linker.define("", "", global.clone()).is_err());

    // memories
    let ty = MemoryType::new(Limits::new(1, None));
    let memory = Memory::new(&store, ty);
    linker.define("", "", memory.clone())?;
    assert!(linker.define("", "", memory.clone()).is_err());
    let ty = MemoryType::new(Limits::new(2, None));
    let memory = Memory::new(&store, ty);
    assert!(linker.define("", "", memory.clone()).is_err());

    // tables
    let ty = TableType::new(ValType::FuncRef, Limits::new(1, None));
    let table = Table::new(&store, ty, Val::FuncRef(None))?;
    linker.define("", "", table.clone())?;
    assert!(linker.define("", "", table.clone()).is_err());
    let ty = TableType::new(ValType::FuncRef, Limits::new(2, None));
    let table = Table::new(&store, ty, Val::FuncRef(None))?;
    assert!(linker.define("", "", table.clone()).is_err());
    Ok(())
}

#[test]
fn function_interposition() -> Result<()> {
    let store = Store::default();
    let mut linker = Linker::new(&store);
    linker.allow_shadowing(true);
    let mut module = Module::new(
        store.engine(),
        r#"(module (func (export "green") (result i32) (i32.const 7)))"#,
    )?;
    for _ in 0..4 {
        let instance = linker.instantiate(&module)?;
        linker.define(
            "red",
            "green",
            instance.get_export("green").unwrap().clone(),
        )?;
        module = Module::new(
            store.engine(),
            r#"(module
                (import "red" "green" (func (result i32)))
                (func (export "green") (result i32) (i32.mul (call 0) (i32.const 2)))
            )"#,
        )?;
    }
    let instance = linker.instantiate(&module)?;
    let func = instance.get_export("green").unwrap().into_func().unwrap();
    let func = func.typed::<(), i32>()?;
    assert_eq!(func.call(())?, 112);
    Ok(())
}

// Same as `function_interposition`, but the linker's name for the function
// differs from the module's name.
#[test]
fn function_interposition_renamed() -> Result<()> {
    let store = Store::default();
    let mut linker = Linker::new(&store);
    linker.allow_shadowing(true);
    let mut module = Module::new(
        store.engine(),
        r#"(module (func (export "export") (result i32) (i32.const 7)))"#,
    )?;
    for _ in 0..4 {
        let instance = linker.instantiate(&module)?;
        linker.define(
            "red",
            "green",
            instance.get_export("export").unwrap().clone(),
        )?;
        module = Module::new(
            store.engine(),
            r#"(module
                (import "red" "green" (func (result i32)))
                (func (export "export") (result i32) (i32.mul (call 0) (i32.const 2)))
            )"#,
        )?;
    }
    let instance = linker.instantiate(&module)?;
    let func = instance.get_func("export").unwrap();
    let func = func.typed::<(), i32>()?;
    assert_eq!(func.call(())?, 112);
    Ok(())
}

// Similar to `function_interposition`, but use `Linker::instance` instead of
// `Linker::define`.
#[test]
fn module_interposition() -> Result<()> {
    let store = Store::default();
    let mut linker = Linker::new(&store);
    linker.allow_shadowing(true);
    let mut module = Module::new(
        store.engine(),
        r#"(module (func (export "export") (result i32) (i32.const 7)))"#,
    )?;
    for _ in 0..4 {
        let instance = linker.instantiate(&module)?;
        linker.instance("instance", &instance)?;
        module = Module::new(
            store.engine(),
            r#"(module
                (import "instance" "export" (func (result i32)))
                (func (export "export") (result i32) (i32.mul (call 0) (i32.const 2)))
            )"#,
        )?;
    }
    let instance = linker.instantiate(&module)?;
    let func = instance.get_export("export").unwrap().into_func().unwrap();
    let func = func.typed::<(), i32>()?;
    assert_eq!(func.call(())?, 112);
    Ok(())
}

#[test]
fn no_leak() -> Result<()> {
    struct DropMe(Rc<Cell<bool>>);

    impl Drop for DropMe {
        fn drop(&mut self) {
            self.0.set(true);
        }
    }

    let flag = Rc::new(Cell::new(false));
    {
        let store = Store::default();
        let mut linker = Linker::new(&store);
        let drop_me = DropMe(flag.clone());
        linker.func("", "", move || drop(&drop_me))?;
        let module = Module::new(
            store.engine(),
            r#"
                (module
                    (func (export "_start"))
                )
            "#,
        )?;
        linker.module("a", &module)?;
    }
    assert!(flag.get(), "store was leaked");
    Ok(())
}

#[test]
fn no_leak_with_imports() -> Result<()> {
    struct DropMe(Rc<Cell<bool>>);

    impl Drop for DropMe {
        fn drop(&mut self) {
            self.0.set(true);
        }
    }

    let flag = Rc::new(Cell::new(false));
    {
        let store = Store::default();
        let mut linker = Linker::new(&store);
        let drop_me = DropMe(flag.clone());
        linker.func("", "", move || drop(&drop_me))?;
        let module = Module::new(
            store.engine(),
            r#"
                (module
                    (import "" "" (func))
                    (func (export "_start"))
                )
            "#,
        )?;
        linker.module("a", &module)?;
    }
    assert!(flag.get(), "store was leaked");
    Ok(())
}

#[test]
fn get_host_function() -> Result<()> {
    let mut config = Config::default();
    config.wrap_host_func("mod", "f1", || {});

    let engine = Engine::new(&config)?;
    let module = Module::new(&engine, r#"(module (import "mod" "f1" (func)))"#)?;
    let store = Store::new(&engine);

    let linker = Linker::new(&store);
    assert!(linker.get(&module.imports().nth(0).unwrap()).is_some());

    Ok(())
}

#[test]
fn shadowing_host_function() -> Result<()> {
    let mut config = Config::default();
    config.wrap_host_func("mod", "f1", || {});

    let engine = Engine::new(&config)?;
    let store = Store::new(&engine);

    let mut linker = Linker::new(&store);
    assert!(linker
        .define("mod", "f1", Func::wrap(&store, || {}))
        .is_err());
    linker.define("mod", "f2", Func::wrap(&store, || {}))?;

    let mut linker = Linker::new(&store);
    linker.allow_shadowing(true);
    linker.define("mod", "f1", Func::wrap(&store, || {}))?;
    linker.define("mod", "f2", Func::wrap(&store, || {}))?;

    Ok(())
}
