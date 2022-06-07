use anyhow::Result;
use wasmtime::component::*;
use wasmtime::{Store, StoreContextMut, Trap};

#[test]
fn can_compile() -> Result<()> {
    let engine = super::engine();
    let libc = r#"
        (module $libc
            (memory (export "memory") 1)
            (func (export "canonical_abi_realloc") (param i32 i32 i32 i32) (result i32)
                unreachable)
            (func (export "canonical_abi_free") (param i32 i32 i32)
                unreachable)
        )
        (instance $libc (instantiate (module $libc)))
    "#;
    Component::new(
        &engine,
        r#"(component
            (import "" (func $f))
            (func (canon.lower (func $f)))
        )"#,
    )?;
    Component::new(
        &engine,
        format!(
            r#"(component
                (import "" (func $f (param string)))
                {libc}
                (func (canon.lower (into $libc) (func $f)))
            )"#
        ),
    )?;
    Component::new(
        &engine,
        format!(
            r#"(component
                (import "f1" (func $f1 (param string) (result string)))
                {libc}
                (func (canon.lower (into $libc) (func $f1)))

                (import "f2" (func $f2 (param u32) (result (list u8))))
                (instance $libc2 (instantiate (module $libc)))
                (func (canon.lower (into $libc2) (func $f2)))

                (func (canon.lower (into $libc2) (func $f1)))
                (func (canon.lower (into $libc) (func $f2)))
            )"#
        ),
    )?;
    Component::new(
        &engine,
        format!(
            r#"(component
                (import "log" (func $log (param string)))
                {libc}
                (func $log_lower (canon.lower (into $libc) (func $log)))

                (module $logger
                    (import "host" "log" (func $log (param i32 i32)))
                    (import "libc" "memory" (memory 1))

                    (func (export "call")
                        i32.const 0
                        i32.const 0
                        call $log)
                )
                (instance $logger (instantiate (module $logger)
                    (with "host" (instance (export "log" (func $log_lower))))
                    (with "libc" (instance $libc))
                ))

                (func (export "call")
                    (canon.lift (func) (func $logger "call"))
                )
            )"#
        ),
    )?;
    Ok(())
}

#[test]
fn simple() -> Result<()> {
    let component = r#"
        (component
            (import "" (func $log (param string)))

            (module $libc
                (memory (export "memory") 1)

                (func (export "canonical_abi_realloc") (param i32 i32 i32 i32) (result i32)
                    unreachable)
                (func (export "canonical_abi_free") (param i32 i32 i32)
                    unreachable)
            )
            (instance $libc (instantiate (module $libc)))
            (func $log_lower
                (canon.lower (into $libc) (func $log))
            )
            (module $m
                (import "libc" "memory" (memory 1))
                (import "host" "log" (func $log (param i32 i32)))

                (func (export "call")
                    i32.const 5
                    i32.const 11
                    call $log)

                (data (i32.const 5) "hello world")
            )
            (instance $i (instantiate (module $m)
                (with "libc" (instance $libc))
                (with "host" (instance (export "log" (func $log_lower))))
            ))
            (func (export "call")
                (canon.lift (func) (func $i "call"))
            )
        )
    "#;

    let engine = super::engine();
    let mut linker = Linker::new(&engine);
    linker.root().func_wrap(
        "",
        |mut store: StoreContextMut<'_, Option<String>>, arg: WasmStr| -> Result<_> {
            let s = arg.to_str(&store)?.to_string();
            assert!(store.data().is_none());
            *store.data_mut() = Some(s);
            Ok(())
        },
    )?;
    let component = Component::new(&engine, component)?;
    let mut store = Store::new(&engine, None);
    let instance = linker.instantiate(&mut store, &component)?;
    assert!(store.data().is_none());
    instance
        .get_typed_func::<(), (), _>(&mut store, "call")?
        .call(&mut store, ())?;
    assert_eq!(store.data().as_ref().unwrap(), "hello world");

    Ok(())
}

#[test]
fn attempt_to_leave_during_malloc() -> Result<()> {
    let component = r#"
(component
  (import "thunk" (func $thunk))
  (import "ret-string" (func $ret_string (result string)))

  (module $host_shim
    (table (export "table") 2 funcref)
    (func $shim_thunk (export "thunk")
      i32.const 0
      call_indirect)
    (func $shim_ret_string (export "ret-string") (param i32)
      local.get 0
      i32.const 1
      call_indirect (param i32))
  )
  (instance $host_shim (instantiate (module $host_shim)))

  (module $m
    (import "host" "thunk" (func $thunk))
    (import "host" "ret-string" (func $ret_string (param i32)))

    (memory (export "memory") 1)

    (func $realloc (export "canonical_abi_realloc") (param i32 i32 i32 i32) (result i32)
      call $thunk
      unreachable)

    (func (export "canonical_abi_free") (param i32 i32 i32)
      unreachable)

    (func $run (export "run")
      i32.const 8
      call $ret_string)

    (func (export "take-string") (param i32 i32)
        unreachable)
  )
  (instance $m (instantiate (module $m) (with "host" (instance $host_shim))))

  (module $host_shim_filler_inner
    (import "shim" "table" (table 2 funcref))
    (import "host" "thunk" (func $thunk))
    (import "host" "ret-string" (func $ret_string (param i32)))
    (elem (i32.const 0) $thunk $ret_string)
  )

  (func $thunk_lower
    (canon.lower (into $m) (func $thunk))
  )

  (func $ret_string_lower
    (canon.lower (into $m) (func $ret_string))
  )

  (instance (instantiate (module $host_shim_filler_inner)
    (with "shim" (instance $host_shim))
    (with "host" (instance
      (export "thunk" (func $thunk_lower))
      (export "ret-string" (func $ret_string_lower))
    ))
  ))

  (func (export "run")
    (canon.lift (func) (func $m "run"))
  )
  (func (export "take-string")
    (canon.lift (func (param string)) (into $m) (func $m "take-string"))
  )
)
    "#;

    let engine = super::engine();
    let mut linker = Linker::new(&engine);
    linker
        .root()
        .func_wrap("thunk", || -> Result<()> { panic!("should not get here") })?;
    linker
        .root()
        .func_wrap("ret-string", || -> Result<String> {
            Ok("hello".to_string())
        })?;
    let component = Component::new(&engine, component)?;
    let mut store = Store::new(&engine, ());
    let instance = linker.instantiate(&mut store, &component)?;

    // Assert that during a host import if we return values to wasm that a trap
    // happens if we try to leave the instance.
    let trap = instance
        .get_typed_func::<(), (), _>(&mut store, "run")?
        .call(&mut store, ())
        .unwrap_err()
        .downcast::<Trap>()?;
    assert!(
        trap.to_string().contains("cannot leave component instance"),
        "bad trap: {}",
        trap,
    );
    let trace = trap.trace().unwrap();
    assert_eq!(trace.len(), 4);

    // This was our entry point...
    assert_eq!(trace[3].module_name(), Some("m"));
    assert_eq!(trace[3].func_name(), Some("run"));

    // ... which called an imported function which ends up being originally
    // defined by the shim instance. The shim instance then does an indirect
    // call through a table which goes to the `canon.lower`'d host function
    assert_eq!(trace[2].module_name(), Some("host_shim"));
    assert_eq!(trace[2].func_name(), Some("shim_ret_string"));

    // ... and the lowered host function will call realloc to allocate space for
    // the result
    assert_eq!(trace[1].module_name(), Some("m"));
    assert_eq!(trace[1].func_name(), Some("realloc"));

    // ... but realloc calls the shim instance and tries to exit the
    // component, triggering a dynamic trap
    assert_eq!(trace[0].module_name(), Some("host_shim"));
    assert_eq!(trace[0].func_name(), Some("shim_thunk"));

    // In addition to the above trap also ensure that when we enter a wasm
    // component if we try to leave while lowering then that's also a dynamic
    // trap.
    let trap = instance
        .get_typed_func::<(&str,), (), _>(&mut store, "take-string")?
        .call(&mut store, ("x",))
        .unwrap_err()
        .downcast::<Trap>()?;
    assert!(
        trap.to_string().contains("cannot leave component instance"),
        "bad trap: {}",
        trap,
    );

    Ok(())
}

#[test]
fn attempt_to_reenter_during_host() -> Result<()> {
    let component = r#"
(component
  (import "thunk" (func $thunk))
  (func $thunk_lower (canon.lower (func $thunk)))

  (module $m
    (import "host" "thunk" (func $thunk))

    (func $run (export "run")
      call $thunk)
  )
  (instance $m (instantiate (module $m)
    (with "host" (instance (export "thunk" (func $thunk_lower))))
  ))

  (func (export "run")
    (canon.lift (func) (func $m "run"))
  )
)
    "#;

    struct State {
        func: Option<TypedFunc<(), ()>>,
    }

    let engine = super::engine();
    let mut linker = Linker::new(&engine);
    linker.root().func_wrap(
        "thunk",
        |mut store: StoreContextMut<'_, State>| -> Result<()> {
            let func = store.data_mut().func.take().unwrap();
            let trap = func.call(&mut store, ()).unwrap_err();
            assert!(
                trap.to_string()
                    .contains("cannot reenter component instance"),
                "bad trap: {}",
                trap,
            );
            Ok(())
        },
    )?;
    let component = Component::new(&engine, component)?;
    let mut store = Store::new(&engine, State { func: None });
    let instance = linker.instantiate(&mut store, &component)?;
    let func = instance.get_typed_func::<(), (), _>(&mut store, "run")?;
    store.data_mut().func = Some(func);
    func.call(&mut store, ())?;
    Ok(())
}
