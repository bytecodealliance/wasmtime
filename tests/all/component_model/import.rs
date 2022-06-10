use super::REALLOC_AND_FREE;
use anyhow::Result;
use wasmtime::component::*;
use wasmtime::{Store, StoreContextMut, Trap};

#[test]
fn can_compile() -> Result<()> {
    let engine = super::engine();
    let libc = r#"
        (core module $libc
            (memory (export "memory") 1)
            (func (export "realloc") (param i32 i32 i32 i32) (result i32)
                unreachable)
        )
        (core instance $libc (instantiate $libc))
    "#;
    Component::new(
        &engine,
        r#"(component
            (import "" (func $f))
            (core func (canon lower (func $f)))
        )"#,
    )?;
    Component::new(
        &engine,
        format!(
            r#"(component
                (import "" (func $f (param string)))
                {libc}
                (core func (canon lower (func $f) (memory $libc "memory") (realloc (func $libc "realloc"))))
            )"#
        ),
    )?;
    Component::new(
        &engine,
        format!(
            r#"(component
                (import "f1" (func $f1 (param string) (result string)))
                {libc}
                (core func (canon lower (func $f1) (memory $libc "memory") (realloc (func $libc "realloc"))))

                (import "f2" (func $f2 (param u32) (result (list u8))))
                (core instance $libc2 (instantiate $libc))
                (core func (canon lower (func $f2) (memory $libc2 "memory") (realloc (func $libc2 "realloc"))))

                (core func (canon lower (func $f1) (memory $libc2 "memory") (realloc (func $libc2 "realloc"))))
                (core func (canon lower (func $f2) (memory $libc "memory") (realloc (func $libc "realloc"))))
            )"#
        ),
    )?;
    Component::new(
        &engine,
        format!(
            r#"(component
                (import "log" (func $log (param string)))
                {libc}
                (core func $log_lower (canon lower (func $log) (memory $libc "memory") (realloc (func $libc "realloc"))))

                (core module $logger
                    (import "host" "log" (func $log (param i32 i32)))
                    (import "libc" "memory" (memory 1))

                    (func (export "call")
                        i32.const 0
                        i32.const 0
                        call $log)
                )
                (core instance $logger (instantiate $logger
                    (with "host" (instance (export "log" (func $log_lower))))
                    (with "libc" (instance $libc))
                ))

                (func (export "call")
                    (canon lift (core func $logger "call"))
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

            (core module $libc
                (memory (export "memory") 1)

                (func (export "realloc") (param i32 i32 i32 i32) (result i32)
                    unreachable)
            )
            (core instance $libc (instantiate $libc))
            (core func $log_lower
                (canon lower (func $log) (memory $libc "memory") (realloc (func $libc "realloc")))
            )
            (core module $m
                (import "libc" "memory" (memory 1))
                (import "host" "log" (func $log (param i32 i32)))

                (func (export "call")
                    i32.const 5
                    i32.const 11
                    call $log)

                (data (i32.const 5) "hello world")
            )
            (core instance $i (instantiate $m
                (with "libc" (instance $libc))
                (with "host" (instance (export "log" (func $log_lower))))
            ))
            (func (export "call")
                (canon lift (core func $i "call"))
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

  (core module $host_shim
    (table (export "table") 2 funcref)
    (func $shim_thunk (export "thunk")
      i32.const 0
      call_indirect)
    (func $shim_ret_string (export "ret-string") (param i32)
      local.get 0
      i32.const 1
      call_indirect (param i32))
  )
  (core instance $host_shim (instantiate $host_shim))

  (core module $m
    (import "host" "thunk" (func $thunk))
    (import "host" "ret-string" (func $ret_string (param i32)))

    (memory (export "memory") 1)

    (func $realloc (export "realloc") (param i32 i32 i32 i32) (result i32)
      call $thunk
      unreachable)

    (func $run (export "run")
      i32.const 8
      call $ret_string)

    (func (export "take-string") (param i32 i32)
        unreachable)
  )
  (core instance $m (instantiate $m (with "host" (instance $host_shim))))

  (core module $host_shim_filler_inner
    (import "shim" "table" (table 2 funcref))
    (import "host" "thunk" (func $thunk))
    (import "host" "ret-string" (func $ret_string (param i32)))
    (elem (i32.const 0) $thunk $ret_string)
  )

  (core func $thunk_lower
    (canon lower (func $thunk) (memory $m "memory") (realloc (func $m "realloc")))
  )

  (core func $ret_string_lower
    (canon lower (func $ret_string) (memory $m "memory") (realloc (func $m "realloc")))
  )

  (core instance (instantiate $host_shim_filler_inner
    (with "shim" (instance $host_shim))
    (with "host" (instance
      (export "thunk" (func $thunk_lower))
      (export "ret-string" (func $ret_string_lower))
    ))
  ))

  (func (export "run")
    (canon lift (core func $m "run"))
  )
  (func (export "take-string") (param string)
    (canon lift (core func $m "take-string") (memory $m "memory") (realloc (func $m "realloc")))
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
  (core func $thunk_lower (canon lower (func $thunk)))

  (core module $m
    (import "host" "thunk" (func $thunk))

    (func $run (export "run")
      call $thunk)
  )
  (core instance $m (instantiate $m
    (with "host" (instance (export "thunk" (func $thunk_lower))))
  ))

  (func (export "run")
    (canon lift (core func $m "run"))
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

#[test]
fn stack_and_heap_args_and_rets() -> Result<()> {
    let component = format!(
        r#"
(component
  (type $many_params (tuple
                      string string string string
                      string string string string
                      string))
  (import "f1" (func $f1 (param u32) (result u32)))
  (import "f2" (func $f2 (param $many_params) (result u32)))
  (import "f3" (func $f3 (param u32) (result string)))
  (import "f4" (func $f4 (param $many_params) (result string)))

  (core module $libc
    {REALLOC_AND_FREE}
    (memory (export "memory") 1)
  )
  (core instance $libc (instantiate (module $libc)))

  (core func $f1_lower (canon lower (func $f1) (memory $libc "memory") (realloc (func $libc "realloc"))))
  (core func $f2_lower (canon lower (func $f2) (memory $libc "memory") (realloc (func $libc "realloc"))))
  (core func $f3_lower (canon lower (func $f3) (memory $libc "memory") (realloc (func $libc "realloc"))))
  (core func $f4_lower (canon lower (func $f4) (memory $libc "memory") (realloc (func $libc "realloc"))))

  (core module $m
    (import "host" "f1" (func $f1 (param i32) (result i32)))
    (import "host" "f2" (func $f2 (param i32) (result i32)))
    (import "host" "f3" (func $f3 (param i32 i32)))
    (import "host" "f4" (func $f4 (param i32 i32)))
    (import "libc" "memory" (memory 1))

    (func $run (export "run")
      block
        i32.const 1
        call $f1
        i32.const 2
        i32.eq
        br_if 0
        unreachable
      end

      block
        call $allocate_empty_strings
        call $f2
        i32.const 3
        i32.eq
        br_if 0
        unreachable
      end

      block
        i32.const 8
        i32.const 16000
        call $f3
        (call $validate_string_ret (i32.const 16000))
      end

      block
        call $allocate_empty_strings
        i32.const 20000
        call $f4
        (call $validate_string_ret (i32.const 20000))
      end
    )

    (func $allocate_empty_strings (result i32)
      (local $ret i32)
      (local $offset i32)
      (local $cnt i32)
      (local.set $ret (i32.const 8000))
      (local.set $cnt (i32.const 9))

      loop
        (call $setup_str (i32.add (local.get $ret) (local.get $offset)))
        (local.set $offset (i32.add (local.get $offset) (i32.const 8)))

        (local.tee $cnt (i32.add (local.get $cnt) (i32.const -1)))
        br_if 0
      end

      local.get $ret
    )
    (func $setup_str (param $addr i32)
      (i32.store offset=0 (local.get $addr) (i32.const 1000))
      (i32.store offset=4 (local.get $addr) (i32.const 3))
    )

    (func $validate_string_ret (param $addr i32)
      (local $base i32)
      (local $len i32)
      (local.set $base (i32.load (local.get $addr)))
      (local.set $len (i32.load offset=4 (local.get $addr)))

      block
        local.get $len
        i32.const 3
        i32.eq
        br_if 0
        unreachable
      end

      (i32.load8_u offset=0 (local.get $base))
      i32.const 120 ;; 'x'
      i32.ne
      if unreachable end

      (i32.load8_u offset=1 (local.get $base))
      i32.const 121 ;; 'y'
      i32.ne
      if unreachable end

      (i32.load8_u offset=2 (local.get $base))
      i32.const 122 ;; 'z'
      i32.ne
      if unreachable end
    )

    (data (i32.const 1000) "abc")
  )
  (core instance $m (instantiate $m
    (with "libc" (instance $libc))
    (with "host" (instance
      (export "f1" (func $f1_lower))
      (export "f2" (func $f2_lower))
      (export "f3" (func $f3_lower))
      (export "f4" (func $f4_lower))
    ))
  ))

  (func (export "run")
    (canon lift (core func $m "run"))
  )
)
        "#
    );

    let engine = super::engine();
    let mut linker = Linker::new(&engine);
    linker.root().func_wrap("f1", |x: u32| -> Result<u32> {
        assert_eq!(x, 1);
        Ok(2)
    })?;
    linker.root().func_wrap(
        "f2",
        |cx: StoreContextMut<'_, ()>,
         arg: (
            WasmStr,
            WasmStr,
            WasmStr,
            WasmStr,
            WasmStr,
            WasmStr,
            WasmStr,
            WasmStr,
            WasmStr,
        )|
         -> Result<u32> {
            assert_eq!(arg.0.to_str(&cx).unwrap(), "abc");
            Ok(3)
        },
    )?;
    linker
        .root()
        .func_wrap("f3", |arg: u32| -> Result<String> {
            assert_eq!(arg, 8);
            Ok("xyz".to_string())
        })?;
    linker.root().func_wrap(
        "f4",
        |cx: StoreContextMut<'_, ()>,
         arg: (
            WasmStr,
            WasmStr,
            WasmStr,
            WasmStr,
            WasmStr,
            WasmStr,
            WasmStr,
            WasmStr,
            WasmStr,
        )|
         -> Result<String> {
            assert_eq!(arg.0.to_str(&cx).unwrap(), "abc");
            Ok("xyz".to_string())
        },
    )?;
    let component = Component::new(&engine, component)?;
    let mut store = Store::new(&engine, ());
    let instance = linker.instantiate(&mut store, &component)?;
    instance
        .get_typed_func::<(), (), _>(&mut store, "run")?
        .call(&mut store, ())?;
    Ok(())
}

#[test]
fn bad_import_alignment() -> Result<()> {
    let component = format!(
        r#"
(component
  (import "unaligned-retptr" (func $unaligned_retptr (result string)))
  (type $many_arg (tuple
    string string string string
    string string string string
    string
  ))
  (import "unaligned-argptr" (func $unaligned_argptr (param $many_arg)))
  (core module $libc_panic
    (memory (export "memory") 1)
    (func (export "realloc") (param i32 i32 i32 i32) (result i32)
      unreachable)
  )
  (core instance $libc_panic (instantiate $libc_panic))

  (core func $unaligned_retptr_lower
    (canon lower (func $unaligned_retptr) (memory $libc_panic "memory") (realloc (func $libc_panic "realloc")))
  )
  (core func $unaligned_argptr_lower
    (canon lower (func $unaligned_argptr) (memory $libc_panic "memory") (realloc (func $libc_panic "realloc")))
  )

  (core module $m
    (import "host" "unaligned-retptr" (func $unaligned_retptr (param i32)))
    (import "host" "unaligned-argptr" (func $unaligned_argptr (param i32)))

    (func (export "unaligned-retptr")
     (call $unaligned_retptr (i32.const 1)))
    (func (export "unaligned-argptr")
     (call $unaligned_argptr (i32.const 1)))
  )
  (core instance $m (instantiate $m
    (with "host" (instance
      (export "unaligned-retptr" (func $unaligned_retptr_lower))
      (export "unaligned-argptr" (func $unaligned_argptr_lower))
    ))
  ))

  (func (export "unaligned-retptr")
    (canon lift (core func $m "unaligned-retptr"))
  )
  (func (export "unaligned-argptr")
    (canon lift (core func $m "unaligned-argptr"))
  )
)
        "#
    );

    let engine = super::engine();
    let mut linker = Linker::new(&engine);
    linker
        .root()
        .func_wrap("unaligned-retptr", || -> Result<String> {
            Ok(String::new())
        })?;
    linker.root().func_wrap(
        "unaligned-argptr",
        |_: (
            WasmStr,
            WasmStr,
            WasmStr,
            WasmStr,
            WasmStr,
            WasmStr,
            WasmStr,
            WasmStr,
            WasmStr,
        )|
         -> Result<()> { unreachable!() },
    )?;
    let component = Component::new(&engine, component)?;
    let mut store = Store::new(&engine, ());
    let instance = linker.instantiate(&mut store, &component)?;
    let trap = instance
        .get_typed_func::<(), (), _>(&mut store, "unaligned-retptr")?
        .call(&mut store, ())
        .unwrap_err()
        .downcast::<Trap>()?;
    assert!(trap.to_string().contains("pointer not aligned"), "{}", trap);
    let trap = instance
        .get_typed_func::<(), (), _>(&mut store, "unaligned-argptr")?
        .call(&mut store, ())
        .unwrap_err()
        .downcast::<Trap>()?;
    assert!(trap.to_string().contains("pointer not aligned"), "{}", trap);
    Ok(())
}
