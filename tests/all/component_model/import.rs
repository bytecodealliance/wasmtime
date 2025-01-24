#![cfg(not(miri))]

use super::REALLOC_AND_FREE;
use anyhow::Result;
use std::ops::Deref;
use wasmtime::component;
use wasmtime::component::*;
use wasmtime::{Config, Engine, Store, StoreContextMut, Trap, WasmBacktrace};

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
            (import "a" (func $f))
            (core func (canon lower (func $f)))
        )"#,
    )?;
    Component::new(
        &engine,
        format!(
            r#"(component
                (import "a" (func $f (param "a" string)))
                {libc}
                (core func (canon lower (func $f) (memory $libc "memory") (realloc (func $libc "realloc"))))
            )"#
        ),
    )?;
    Component::new(
        &engine,
        format!(
            r#"(component
                (import "f1" (func $f1 (param "a" string) (result string)))
                {libc}
                (core func (canon lower (func $f1) (memory $libc "memory") (realloc (func $libc "realloc"))))

                (import "f2" (func $f2 (param "a" u32) (result (list u8))))
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
                (import "log" (func $log (param "a" string)))
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
            (import "a" (func $log (param "a" string)))

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
    let component = Component::new(&engine, component)?;
    let mut store = Store::new(&engine, None);
    assert!(store.data().is_none());

    // First, test the static API

    let mut linker = Linker::new(&engine);
    linker.root().func_wrap(
        "a",
        |mut store: StoreContextMut<'_, Option<String>>, (arg,): (WasmStr,)| -> Result<_> {
            let s = arg.to_str(&store)?.to_string();
            assert!(store.data().is_none());
            *store.data_mut() = Some(s);
            Ok(())
        },
    )?;
    let instance = linker.instantiate(&mut store, &component)?;
    instance
        .get_typed_func::<(), ()>(&mut store, "call")?
        .call(&mut store, ())?;
    assert_eq!(store.data().as_ref().unwrap(), "hello world");

    // Next, test the dynamic API

    *store.data_mut() = None;
    let mut linker = Linker::new(&engine);
    linker.root().func_new(
        "a",
        |mut store: StoreContextMut<'_, Option<String>>, args, _results| {
            if let Val::String(s) = &args[0] {
                assert!(store.data().is_none());
                *store.data_mut() = Some(s.to_string());
                Ok(())
            } else {
                panic!()
            }
        },
    )?;
    let instance = linker.instantiate(&mut store, &component)?;
    instance
        .get_func(&mut store, "call")
        .unwrap()
        .call(&mut store, &[], &mut [])?;
    assert_eq!(store.data().as_ref().unwrap(), "hello world");

    Ok(())
}

#[test]
fn functions_in_instances() -> Result<()> {
    let component = r#"
        (component
            (type $import-type (instance
                (export "a" (func (param "a" string)))
            ))
            (import (interface "test:test/foo") (instance $import (type $import-type)))
            (alias export $import "a" (func $log))

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
            (func $call
                (canon lift (core func $i "call"))
            )
            (component $c
                (import "import-call" (func $f))
                (export "call" (func $f))
            )
            (instance $export (instantiate $c
                (with "import-call" (func $call))
            ))
            (export (interface "test:test/foo") (instance $export))
        )
    "#;

    let engine = super::engine();
    let component = Component::new(&engine, component)?;
    let (_, instance_index) = component.export_index(None, "test:test/foo").unwrap();
    let (_, func_index) = component
        .export_index(Some(&instance_index), "call")
        .unwrap();
    let mut store = Store::new(&engine, None);
    assert!(store.data().is_none());

    // First, test the static API

    let mut linker = Linker::new(&engine);
    linker.instance("test:test/foo")?.func_wrap(
        "a",
        |mut store: StoreContextMut<'_, Option<String>>, (arg,): (WasmStr,)| -> Result<_> {
            let s = arg.to_str(&store)?.to_string();
            assert!(store.data().is_none());
            *store.data_mut() = Some(s);
            Ok(())
        },
    )?;
    let instance = linker.instantiate(&mut store, &component)?;
    let func = instance.get_typed_func::<(), ()>(&mut store, &func_index)?;
    func.call(&mut store, ())?;
    assert_eq!(store.data().as_ref().unwrap(), "hello world");

    // Next, test the dynamic API

    *store.data_mut() = None;
    let mut linker = Linker::new(&engine);
    linker.instance("test:test/foo")?.func_new(
        "a",
        |mut store: StoreContextMut<'_, Option<String>>, args, _results| {
            if let Val::String(s) = &args[0] {
                assert!(store.data().is_none());
                *store.data_mut() = Some(s.to_string());
                Ok(())
            } else {
                panic!()
            }
        },
    )?;
    let instance = linker.instantiate(&mut store, &component)?;
    let func = instance.get_func(&mut store, func_index).unwrap();
    func.call(&mut store, &[], &mut [])?;
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
  (func (export "take-string") (param "a" string)
    (canon lift (core func $m "take-string") (memory $m "memory") (realloc (func $m "realloc")))
  )
)
    "#;

    let engine = super::engine();
    let mut linker = Linker::new(&engine);
    linker.root().func_wrap("thunk", |_, _: ()| -> Result<()> {
        panic!("should not get here")
    })?;
    linker
        .root()
        .func_wrap("ret-string", |_, _: ()| -> Result<_> {
            Ok(("hello".to_string(),))
        })?;
    let component = Component::new(&engine, component)?;
    let mut store = Store::new(&engine, ());

    // Assert that during a host import if we return values to wasm that a trap
    // happens if we try to leave the instance.
    let trap = linker
        .instantiate(&mut store, &component)?
        .get_typed_func::<(), ()>(&mut store, "run")?
        .call(&mut store, ())
        .unwrap_err();
    assert!(
        format!("{trap:?}").contains("cannot leave component instance"),
        "bad trap: {trap:?}",
    );

    let trace = trap.downcast_ref::<WasmBacktrace>().unwrap().frames();
    assert_eq!(trace.len(), 4);

    // This was our entry point...
    assert_eq!(trace[3].module().name(), Some("m"));
    assert_eq!(trace[3].func_name(), Some("run"));

    // ... which called an imported function which ends up being originally
    // defined by the shim instance. The shim instance then does an indirect
    // call through a table which goes to the `canon.lower`'d host function
    assert_eq!(trace[2].module().name(), Some("host_shim"));
    assert_eq!(trace[2].func_name(), Some("shim_ret_string"));

    // ... and the lowered host function will call realloc to allocate space for
    // the result
    assert_eq!(trace[1].module().name(), Some("m"));
    assert_eq!(trace[1].func_name(), Some("realloc"));

    // ... but realloc calls the shim instance and tries to exit the
    // component, triggering a dynamic trap
    assert_eq!(trace[0].module().name(), Some("host_shim"));
    assert_eq!(trace[0].func_name(), Some("shim_thunk"));

    // In addition to the above trap also ensure that when we enter a wasm
    // component if we try to leave while lowering then that's also a dynamic
    // trap.
    let trap = linker
        .instantiate(&mut store, &component)?
        .get_typed_func::<(&str,), ()>(&mut store, "take-string")?
        .call(&mut store, ("x",))
        .unwrap_err();
    assert!(
        format!("{trap:?}").contains("cannot leave component instance"),
        "bad trap: {trap:?}",
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

    let engine = super::engine();
    let component = Component::new(&engine, component)?;

    // First, test the static API

    struct StaticState {
        func: Option<TypedFunc<(), ()>>,
    }

    let mut store = Store::new(&engine, StaticState { func: None });
    let mut linker = Linker::new(&engine);
    linker.root().func_wrap(
        "thunk",
        |mut store: StoreContextMut<'_, StaticState>, _: ()| -> Result<()> {
            let func = store.data_mut().func.take().unwrap();
            let trap = func.call(&mut store, ()).unwrap_err();
            assert_eq!(
                trap.downcast_ref(),
                Some(&Trap::CannotEnterComponent),
                "bad trap: {trap:?}",
            );
            Ok(())
        },
    )?;
    let instance = linker.instantiate(&mut store, &component)?;
    let func = instance.get_typed_func::<(), ()>(&mut store, "run")?;
    store.data_mut().func = Some(func);
    func.call(&mut store, ())?;

    // Next, test the dynamic API

    struct DynamicState {
        func: Option<Func>,
    }

    let mut store = Store::new(&engine, DynamicState { func: None });
    let mut linker = Linker::new(&engine);
    linker.root().func_new(
        "thunk",
        |mut store: StoreContextMut<'_, DynamicState>, _, _| {
            let func = store.data_mut().func.take().unwrap();
            let trap = func.call(&mut store, &[], &mut []).unwrap_err();
            assert_eq!(
                trap.downcast_ref(),
                Some(&Trap::CannotEnterComponent),
                "bad trap: {trap:?}",
            );
            Ok(())
        },
    )?;
    let instance = linker.instantiate(&mut store, &component)?;
    let func = instance.get_func(&mut store, "run").unwrap();
    store.data_mut().func = Some(func);
    func.call(&mut store, &[], &mut [])?;

    Ok(())
}

#[tokio::test]
async fn stack_and_heap_args_and_rets() -> Result<()> {
    test_stack_and_heap_args_and_rets(false).await
}

#[tokio::test]
async fn stack_and_heap_args_and_rets_concurrent() -> Result<()> {
    test_stack_and_heap_args_and_rets(true).await
}

async fn test_stack_and_heap_args_and_rets(concurrent: bool) -> Result<()> {
    let (body, async_lower_opts, async_lift_opts) = if concurrent {
        (
            r#"
    (import "host" "f1" (func $f1 (param i32 i32) (result i32)))
    (import "host" "f2" (func $f2 (param i32 i32) (result i32)))
    (import "host" "f3" (func $f3 (param i32 i32) (result i32)))
    (import "host" "f4" (func $f4 (param i32 i32) (result i32)))

    (func $run (export "run") (result i32)
      (local $params i32)
      (local $results i32)

      block
        (local.set $params (call $realloc (i32.const 0) (i32.const 0) (i32.const 4) (i32.const 4)))
        (i32.store offset=0 (local.get $params) (i32.const 1))
        (local.set $results (call $realloc (i32.const 0) (i32.const 0) (i32.const 4) (i32.const 4)))
        (call $f1 (local.get $params) (local.get $results))
        drop
        (i32.load offset=0 (local.get $results))
        i32.const 2
        i32.eq
        br_if 0
        unreachable
      end

      block
        (local.set $params (call $allocate_empty_strings))
        (local.set $results (call $realloc (i32.const 0) (i32.const 0) (i32.const 4) (i32.const 4)))
        (call $f2 (local.get $params) (local.get $results))
        drop
        (i32.load offset=0 (local.get $results))
        i32.const 3
        i32.eq
        br_if 0
        unreachable
      end

      block
        (local.set $params (call $realloc (i32.const 0) (i32.const 0) (i32.const 4) (i32.const 4)))
        (i32.store offset=0 (local.get $params) (i32.const 8))
        (local.set $results (call $realloc (i32.const 0) (i32.const 0) (i32.const 4) (i32.const 8)))
        (call $f3 (local.get $params) (local.get $results))
        drop
        (call $validate_string_ret (local.get $results))
      end

      block
        (local.set $params (call $allocate_empty_strings))
        (local.set $results (call $realloc (i32.const 0) (i32.const 0) (i32.const 4) (i32.const 8)))
        (call $f4 (local.get $params) (local.get $results))
        drop
        (call $validate_string_ret (local.get $results))
      end

      (call $task-return)

      i32.const 0
    )
            "#,
            "async",
            r#"async (callback (func $m "callback"))"#,
        )
    } else {
        (
            r#"
    (import "host" "f1" (func $f1 (param i32) (result i32)))
    (import "host" "f2" (func $f2 (param i32) (result i32)))
    (import "host" "f3" (func $f3 (param i32 i32)))
    (import "host" "f4" (func $f4 (param i32 i32)))

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
            "#,
            "",
            "",
        )
    };

    let component = format!(
        r#"
(component
  (type $many_params (tuple
                      string string string string
                      string string string string
                      string))
  (import "f1" (func $f1 (param "a" u32) (result u32)))
  (import "f2" (func $f2 (param "a" $many_params) (result u32)))
  (import "f3" (func $f3 (param "a" u32) (result string)))
  (import "f4" (func $f4 (param "a" $many_params) (result string)))

  (core module $libc
    {REALLOC_AND_FREE}
    (memory (export "memory") 1)
  )
  (core instance $libc (instantiate (module $libc)))

  (core func $f1_lower (canon lower (func $f1)
      (memory $libc "memory")
      (realloc (func $libc "realloc"))
      {async_lower_opts}
  ))
  (core func $f2_lower (canon lower (func $f2)
      (memory $libc "memory")
      (realloc (func $libc "realloc"))
      {async_lower_opts}
  ))
  (core func $f3_lower (canon lower (func $f3)
      (memory $libc "memory")
      (realloc (func $libc "realloc"))
      {async_lower_opts}
  ))
  (core func $f4_lower (canon lower (func $f4)
      (memory $libc "memory")
      (realloc (func $libc "realloc"))
      {async_lower_opts}
  ))

  (core module $m
    (import "libc" "memory" (memory 1))
    (import "libc" "realloc" (func $realloc (param i32 i32 i32 i32) (result i32)))
    (import "host" "task.return" (func $task-return))
    {body}

    (func (export "callback") (param i32 i32 i32 i32) (result i32) unreachable)

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
  (core func $task-return (canon task.return))
  (core instance $m (instantiate $m
    (with "libc" (instance $libc))
    (with "host" (instance
      (export "f1" (func $f1_lower))
      (export "f2" (func $f2_lower))
      (export "f3" (func $f3_lower))
      (export "f4" (func $f4_lower))
      (export "task.return" (func $task-return))
    ))
  ))

  (func (export "run")
    (canon lift (core func $m "run") {async_lift_opts})
  )
)
        "#
    );

    let mut config = Config::new();
    config.wasm_component_model_async(true);
    config.async_support(true);
    let engine = &Engine::new(&config)?;
    let component = Component::new(&engine, component)?;
    let mut store = Store::new(&engine, ());

    // First, test the static API

    let mut linker = Linker::new(&engine);
    if concurrent {
        linker
            .root()
            .func_wrap_concurrent("f1", |_, (x,): (u32,)| {
                assert_eq!(x, 1);
                async { component::for_any(|_| Ok((2u32,))) }
            })?;
        linker.root().func_wrap_concurrent(
            "f2",
            |cx: StoreContextMut<'_, ()>,
             (arg,): ((
                WasmStr,
                WasmStr,
                WasmStr,
                WasmStr,
                WasmStr,
                WasmStr,
                WasmStr,
                WasmStr,
                WasmStr,
            ),)| {
                assert_eq!(arg.0.to_str(&cx).unwrap(), "abc");
                async { component::for_any(|_| Ok((3u32,))) }
            },
        )?;
        linker
            .root()
            .func_wrap_concurrent("f3", |_, (arg,): (u32,)| {
                assert_eq!(arg, 8);
                async { component::for_any(|_| Ok(("xyz".to_string(),))) }
            })?;
        linker.root().func_wrap_concurrent(
            "f4",
            |cx: StoreContextMut<'_, ()>,
             (arg,): ((
                WasmStr,
                WasmStr,
                WasmStr,
                WasmStr,
                WasmStr,
                WasmStr,
                WasmStr,
                WasmStr,
                WasmStr,
            ),)| {
                assert_eq!(arg.0.to_str(&cx).unwrap(), "abc");
                async { component::for_any(|_| Ok(("xyz".to_string(),))) }
            },
        )?;
    } else {
        linker
            .root()
            .func_wrap("f1", |_, (x,): (u32,)| -> Result<(u32,)> {
                assert_eq!(x, 1);
                Ok((2,))
            })?;
        linker.root().func_wrap(
            "f2",
            |cx: StoreContextMut<'_, ()>,
             (arg,): ((
                WasmStr,
                WasmStr,
                WasmStr,
                WasmStr,
                WasmStr,
                WasmStr,
                WasmStr,
                WasmStr,
                WasmStr,
            ),)|
             -> Result<(u32,)> {
                assert_eq!(arg.0.to_str(&cx).unwrap(), "abc");
                Ok((3,))
            },
        )?;
        linker
            .root()
            .func_wrap("f3", |_, (arg,): (u32,)| -> Result<(String,)> {
                assert_eq!(arg, 8);
                Ok(("xyz".to_string(),))
            })?;
        linker.root().func_wrap(
            "f4",
            |cx: StoreContextMut<'_, ()>,
             (arg,): ((
                WasmStr,
                WasmStr,
                WasmStr,
                WasmStr,
                WasmStr,
                WasmStr,
                WasmStr,
                WasmStr,
                WasmStr,
            ),)|
             -> Result<(String,)> {
                assert_eq!(arg.0.to_str(&cx).unwrap(), "abc");
                Ok(("xyz".to_string(),))
            },
        )?;
    }

    let instance = linker.instantiate_async(&mut store, &component).await?;
    let run = instance.get_typed_func::<(), ()>(&mut store, "run")?;

    if concurrent {
        let promise = run.call_concurrent(&mut store, ()).await?;
        promise.get(&mut store).await?;
    } else {
        run.call_async(&mut store, ()).await?;
    }

    // Next, test the dynamic API

    let mut linker = Linker::new(&engine);
    if concurrent {
        linker.root().func_new_concurrent("f1", |_, args| {
            if let Val::U32(x) = &args[0] {
                assert_eq!(*x, 1);
                async { component::for_any(|_| Ok(vec![Val::U32(2)])) }
            } else {
                panic!()
            }
        })?;
        linker.root().func_new_concurrent("f2", |_, args| {
            if let Val::Tuple(tuple) = &args[0] {
                if let Val::String(s) = &tuple[0] {
                    assert_eq!(s.deref(), "abc");
                    async { component::for_any(|_| Ok(vec![Val::U32(3)])) }
                } else {
                    panic!()
                }
            } else {
                panic!()
            }
        })?;
        linker.root().func_new_concurrent("f3", |_, args| {
            if let Val::U32(x) = &args[0] {
                assert_eq!(*x, 8);
                async { component::for_any(|_| Ok(vec![Val::String("xyz".into())])) }
            } else {
                panic!();
            }
        })?;
        linker.root().func_new_concurrent("f4", |_, args| {
            if let Val::Tuple(tuple) = &args[0] {
                if let Val::String(s) = &tuple[0] {
                    assert_eq!(s.deref(), "abc");
                    async { component::for_any(|_| Ok(vec![Val::String("xyz".into())])) }
                } else {
                    panic!()
                }
            } else {
                panic!()
            }
        })?;
    } else {
        linker.root().func_new("f1", |_, args, results| {
            if let Val::U32(x) = &args[0] {
                assert_eq!(*x, 1);
                results[0] = Val::U32(2);
                Ok(())
            } else {
                panic!()
            }
        })?;
        linker.root().func_new("f2", |_, args, results| {
            if let Val::Tuple(tuple) = &args[0] {
                if let Val::String(s) = &tuple[0] {
                    assert_eq!(s.deref(), "abc");
                    results[0] = Val::U32(3);
                    Ok(())
                } else {
                    panic!()
                }
            } else {
                panic!()
            }
        })?;
        linker.root().func_new("f3", |_, args, results| {
            if let Val::U32(x) = &args[0] {
                assert_eq!(*x, 8);
                results[0] = Val::String("xyz".into());
                Ok(())
            } else {
                panic!();
            }
        })?;
        linker.root().func_new("f4", |_, args, results| {
            if let Val::Tuple(tuple) = &args[0] {
                if let Val::String(s) = &tuple[0] {
                    assert_eq!(s.deref(), "abc");
                    results[0] = Val::String("xyz".into());
                    Ok(())
                } else {
                    panic!()
                }
            } else {
                panic!()
            }
        })?;
    }

    let instance = linker.instantiate_async(&mut store, &component).await?;
    let run = instance.get_func(&mut store, "run").unwrap();

    if concurrent {
        let promise = run.call_concurrent(&mut store, Vec::new()).await?;
        promise.get(&mut store).await?;
    } else {
        run.call_async(&mut store, &[], &mut []).await?;
    }

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
  (import "unaligned-argptr" (func $unaligned_argptr (param "a" $many_arg)))
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

  (func (export "unaligned-retptr2")
    (canon lift (core func $m "unaligned-retptr"))
  )
  (func (export "unaligned-argptr2")
    (canon lift (core func $m "unaligned-argptr"))
  )
)
        "#
    );

    let engine = super::engine();
    let mut linker = Linker::new(&engine);
    linker
        .root()
        .func_wrap("unaligned-retptr", |_, _: ()| -> Result<(String,)> {
            Ok((String::new(),))
        })?;
    linker.root().func_wrap(
        "unaligned-argptr",
        |_,
         _: ((
            WasmStr,
            WasmStr,
            WasmStr,
            WasmStr,
            WasmStr,
            WasmStr,
            WasmStr,
            WasmStr,
            WasmStr,
        ),)|
         -> Result<()> { unreachable!() },
    )?;
    let component = Component::new(&engine, component)?;
    let mut store = Store::new(&engine, ());

    let trap = linker
        .instantiate(&mut store, &component)?
        .get_typed_func::<(), ()>(&mut store, "unaligned-retptr2")?
        .call(&mut store, ())
        .unwrap_err();
    assert!(
        format!("{trap:?}").contains("pointer not aligned"),
        "{}",
        trap
    );
    let trap = linker
        .instantiate(&mut store, &component)?
        .get_typed_func::<(), ()>(&mut store, "unaligned-argptr2")?
        .call(&mut store, ())
        .unwrap_err();
    assert!(
        format!("{trap:?}").contains("pointer not aligned"),
        "{}",
        trap
    );

    Ok(())
}

#[test]
fn no_actual_wasm_code() -> Result<()> {
    let component = r#"
        (component
            (import "f" (func $f))

            (core func $f_lower
                (canon lower (func $f))
            )
            (core module $m
                (import "" "" (func $f))
                (export "f" (func $f))
            )
            (core instance $i (instantiate $m
                (with "" (instance
                    (export "" (func $f_lower))
                ))
            ))
            (func (export "thunk")
                (canon lift
                    (core func $i "f")
                )
            )
        )
    "#;

    let engine = super::engine();
    let component = Component::new(&engine, component)?;
    let mut store = Store::new(&engine, 0);

    // First, test the static API

    let mut linker = Linker::new(&engine);
    linker.root().func_wrap(
        "f",
        |mut store: StoreContextMut<'_, u32>, _: ()| -> Result<()> {
            *store.data_mut() += 1;
            Ok(())
        },
    )?;

    let instance = linker.instantiate(&mut store, &component)?;
    let thunk = instance.get_typed_func::<(), ()>(&mut store, "thunk")?;

    assert_eq!(*store.data(), 0);
    thunk.call(&mut store, ())?;
    assert_eq!(*store.data(), 1);

    // Next, test the dynamic API

    *store.data_mut() = 0;
    let mut linker = Linker::new(&engine);
    linker
        .root()
        .func_new("f", |mut store: StoreContextMut<'_, u32>, _, _| {
            *store.data_mut() += 1;
            Ok(())
        })?;

    let instance = linker.instantiate(&mut store, &component)?;
    let thunk = instance.get_func(&mut store, "thunk").unwrap();

    assert_eq!(*store.data(), 0);
    thunk.call(&mut store, &[], &mut [])?;
    assert_eq!(*store.data(), 1);

    Ok(())
}

#[test]
fn use_types_across_component_boundaries() -> Result<()> {
    // Create a component that exports a function that returns a record
    let engine = super::engine();
    let component = Component::new(
        &engine,
        r#"(component
            (type (;0;) (record (field "a" u8) (field "b" string)))
            (import "my-record" (type $my-record (eq 0)))
            (core module $m
                (memory $memory 17)
                (export "memory" (memory $memory))
                (func (export "my-func") (result i32)
                    i32.const 4
                    return))
            (core instance $instance (instantiate $m))
            (type $func-type (func (result $my-record)))
            (alias core export $instance "my-func" (core func $my-func))
            (alias core export $instance "memory" (core memory $memory))
            (func $my-func (type $func-type) (canon lift (core func $my-func) (memory $memory) string-encoding=utf8))
            (export $export "my-func" (func $my-func))
        )"#,
    )?;
    let mut store = Store::new(&engine, 0);
    let linker = Linker::new(&engine);
    let instance = linker.instantiate(&mut store, &component)?;
    let my_func = instance.get_func(&mut store, "my-func").unwrap();
    let mut results = vec![Val::Bool(false)];
    my_func.call(&mut store, &[], &mut results)?;

    // Create another component that exports a function that takes that record as an argument
    let component = Component::new(
        &engine,
        format!(
            r#"(component
            (type (;0;) (record (field "a" u8) (field "b" string)))
            (import "my-record" (type $my-record (eq 0)))
            (core module $m
                (memory $memory 17)
                (export "memory" (memory $memory))
                {REALLOC_AND_FREE}
                (func (export "my-func") (param i32 i32 i32)))
            (core instance $instance (instantiate $m))
            (type $func-type (func (param "my-record" $my-record)))
            (alias core export $instance "my-func" (core func $my-func))
            (alias core export $instance "memory" (core memory $memory))
            (func $my-func (type $func-type) (canon lift (core func $my-func) (memory $memory) string-encoding=utf8 (realloc (func $instance "realloc"))))
            (export $export "my-func" (func $my-func))
        )"#
        ),
    )?;
    let mut store = Store::new(&engine, 0);
    let linker = Linker::new(&engine);
    let instance = linker.instantiate(&mut store, &component)?;
    let my_func = instance.get_func(&mut store, "my-func").unwrap();
    // Call the exported function with the return values of the call to the previous component's exported function
    my_func.call(&mut store, &results, &mut [])?;

    Ok(())
}
