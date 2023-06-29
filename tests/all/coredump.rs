use anyhow::{bail, Result};
use wasmtime::*;

#[test]
fn test_coredump_attached_to_error() -> Result<()> {
    let mut config = Config::default();
    config.coredump_on_trap(true);
    let engine = Engine::new(&config).unwrap();
    let mut store = Store::<()>::new(&engine, ());

    let wat = r#"
      (module
      (func $hello (import "" "hello"))
      (func (export "run") (call $hello))
      )
    "#;

    let module = Module::new(store.engine(), wat)?;
    let hello_type = FuncType::new(None, None);
    let hello_func = Func::new(&mut store, hello_type, |_, _, _| bail!("test 123"));

    let instance = Instance::new(&mut store, &module, &[hello_func.into()])?;
    let run_func = instance.get_typed_func::<(), ()>(&mut store, "run")?;

    let e = run_func.call(&mut store, ()).unwrap_err();
    assert!(format!("{e:?}").contains("test 123"));

    assert!(
        e.downcast_ref::<WasmCoreDump>().is_some(),
        "error should contain a WasmCoreDump"
    );

    Ok(())
}

#[test]
fn test_coredump_has_stack() -> Result<()> {
    let mut config = Config::default();
    config.coredump_on_trap(true);
    let engine = Engine::new(&config).unwrap();
    let mut store = Store::<()>::new(&engine, ());

    let wat = r#"
      (module
        (func $a (export "a")
            call $b
        )
        (func $b
            call $c
        )
        (func $c 
            unreachable
        )
      )
    "#;

    let module = Module::new(store.engine(), wat)?;
    let instance = Instance::new(&mut store, &module, &[])?;
    let a_func = instance.get_typed_func::<(), ()>(&mut store, "a")?;

    let e = a_func.call(&mut store, ()).unwrap_err();
    let cd = e.downcast_ref::<WasmCoreDump>().unwrap();
    assert_eq!(cd.frames().len(), 3);
    assert_eq!(cd.frames()[0].func_name().unwrap(), "c");
    assert_eq!(cd.frames()[1].func_name().unwrap(), "b");
    assert_eq!(cd.frames()[2].func_name().unwrap(), "a");
    Ok(())
}

#[test]
fn test_coredump_has_modules_and_instances() -> Result<()> {
    let mut config = Config::default();
    config.coredump_on_trap(true);
    let engine = Engine::new(&config).unwrap();
    let mut linker = Linker::new(&engine);
    let mut store = Store::<()>::new(&engine, ());

    let wat1 = r#"
      (module $foo
        (import "bar" "b" (func $b))
        (func (export "a")
            call $b
        )
      )
    "#;
    let wat2 = r#"
      (module $bar
        (func (export "b")
            unreachable
        )
      )
    "#;
    let module1 = Module::new(store.engine(), wat1)?;
    let module2 = Module::new(store.engine(), wat2)?;
    let linking2 = linker.instantiate(&mut store, &module2)?;
    linker.instance(&mut store, "bar", linking2)?;

    let linking1 = linker.instantiate(&mut store, &module1)?;
    let a_func = linking1.get_typed_func::<(), ()>(&mut store, "a")?;

    let e = a_func.call(&mut store, ()).unwrap_err();
    let cd = e.downcast_ref::<WasmCoreDump>().unwrap();
    assert_eq!(cd.modules().len(), 2);
    assert!(cd.modules().contains(&String::from("foo")));
    assert!(cd.modules().contains(&String::from("bar")));
    assert_eq!(cd.instances().len(), 2);
    Ok(())
}

#[test]
fn test_coredump_has_globals_and_memory() -> Result<()> {
    let mut config = Config::default();
    config.coredump_on_trap(true);
    let engine = Engine::new(&config).unwrap();
    let mut store = Store::<()>::new(&engine, ());

    let wat = r#"
      (module
        (memory (export "memory") 2)
        (global (export "myglobal") i32 (i32.const 65536))
        (func (export "a") (result i32)
          unreachable
        )
      )
    "#;

    let module = Module::new(store.engine(), wat)?;
    let instance = Instance::new(&mut store, &module, &[])?;
    let a_func = instance.get_typed_func::<(), i32>(&mut store, "a")?;
    let e = a_func.call(&mut store, ()).unwrap_err();
    let cd = e.downcast_ref::<WasmCoreDump>().unwrap();
    assert_eq!(cd.globals().len(), 1);
    assert_eq!(cd.memories().len(), 1);

    Ok(())
}
