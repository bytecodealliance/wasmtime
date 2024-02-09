use anyhow::{bail, Result};
use wasmtime::*;

#[test]
#[cfg_attr(miri, ignore)]
fn coredump_attached_to_error() -> Result<()> {
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
    let hello_type = FuncType::new(store.engine(), None, None);
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
#[cfg_attr(miri, ignore)]
fn coredump_has_stack() -> Result<()> {
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
    let _ = cd.serialize(&mut store, "stack");
    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn coredump_has_modules_and_instances() -> Result<()> {
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
    assert_eq!(cd.instances().len(), 2);
    let _ = cd.serialize(&mut store, "modules-and-instances");
    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn coredump_has_host_globals_and_memory() -> Result<()> {
    let mut config = Config::default();
    config.coredump_on_trap(true);
    let engine = Engine::new(&config).unwrap();

    let module = Module::new(
        &engine,
        r#"
            (module
                (import "memory" "memory" (memory 1))
                (global $myglobal (import "global" "global") (mut i32))
                (func (export "a") (result i32)
                    unreachable
                )
                (export "memory" (memory 0))
                (export "global" (global 0))
            )
        "#,
    )?;

    let mut store = Store::<()>::new(&engine, ());
    let mut linker = Linker::new(&engine);

    let memory = Memory::new(&mut store, MemoryType::new(1, None))?;
    linker.define(&mut store, "memory", "memory", memory)?;

    let global = Global::new(
        &mut store,
        GlobalType::new(ValType::I32, Mutability::Var),
        Val::I32(0),
    )?;
    linker.define(&mut store, "global", "global", global)?;

    let instance = linker.instantiate(&mut store, &module)?;

    // Each time we extract the exports, it puts them in the `StoreData`. Our
    // core dumps need to be robust to duplicate entries in the `StoreData`.
    for _ in 0..10 {
        let _ = instance.get_global(&mut store, "global").unwrap();
        let _ = instance.get_memory(&mut store, "memory").unwrap();
    }

    let a_func = instance.get_typed_func::<(), i32>(&mut store, "a")?;
    let err = a_func.call(&mut store, ()).unwrap_err();
    let core_dump = err.downcast_ref::<WasmCoreDump>().unwrap();
    assert_eq!(core_dump.globals().len(), 1);
    assert_eq!(core_dump.memories().len(), 1);
    assert_eq!(core_dump.instances().len(), 1);
    let _ = core_dump.serialize(&mut store, "host-globals-and-memory");

    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn coredump_has_defined_globals_and_memory() -> Result<()> {
    let mut config = Config::default();
    config.coredump_on_trap(true);
    let engine = Engine::new(&config).unwrap();

    let module = Module::new(
        &engine,
        r#"
            (module
                (global (mut i32) (i32.const 42))
                (memory 1)
                (func (export "a")
                    unreachable
                )
            )
        "#,
    )?;

    let mut store = Store::<()>::new(&engine, ());
    let instance = Instance::new(&mut store, &module, &[])?;

    let a_func = instance.get_typed_func::<(), ()>(&mut store, "a")?;
    let err = a_func.call(&mut store, ()).unwrap_err();
    let core_dump = err.downcast_ref::<WasmCoreDump>().unwrap();
    assert_eq!(core_dump.globals().len(), 1);
    assert_eq!(core_dump.memories().len(), 1);
    assert_eq!(core_dump.instances().len(), 1);
    let _ = core_dump.serialize(&mut store, "defined-globals-and-memory");

    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn multiple_globals_memories_and_instances() -> Result<()> {
    let mut config = Config::default();
    config.wasm_multi_memory(true);
    config.coredump_on_trap(true);
    let engine = Engine::new(&config).unwrap();
    let mut store = Store::<()>::new(&engine, ());
    let mut linker = Linker::new(&engine);

    let memory = Memory::new(&mut store, MemoryType::new(1, None))?;
    linker.define(&mut store, "host", "memory", memory)?;

    let global = Global::new(
        &mut store,
        GlobalType::new(ValType::I32, Mutability::Var),
        Val::I32(0),
    )?;
    linker.define(&mut store, "host", "global", global)?;

    let module_a = Module::new(
        &engine,
        r#"
            (module
                (memory (export "memory") 1)
                (global (export "global") (mut i32) (i32.const 0))
            )
        "#,
    )?;
    let instance_a = linker.instantiate(&mut store, &module_a)?;
    linker.instance(&mut store, "a", instance_a)?;

    let module_b = Module::new(
        &engine,
        r#"
            (module
                (import "host" "memory" (memory 1))
                (import "host" "global" (global (mut i32)))
                (import "a" "memory" (memory 1))
                (import "a" "global" (global (mut i32)))

                (func (export "trap")
                    unreachable
                )
            )
        "#,
    )?;
    let instance_b = linker.instantiate(&mut store, &module_b)?;

    let trap_func = instance_b.get_typed_func::<(), ()>(&mut store, "trap")?;
    let err = trap_func.call(&mut store, ()).unwrap_err();
    let core_dump = err.downcast_ref::<WasmCoreDump>().unwrap();
    assert_eq!(core_dump.globals().len(), 2);
    assert_eq!(core_dump.memories().len(), 2);
    assert_eq!(core_dump.instances().len(), 2);
    let _ = core_dump.serialize(&mut store, "multi");

    Ok(())
}
