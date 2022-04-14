use anyhow::Error;
use std::future::Future;
use std::pin::Pin;
use std::task::{self, Poll};
use wasmtime::*;

// Crate a synchronous Func, call it directly:
#[test]
fn call_wrapped_func() -> Result<(), Error> {
    let mut store = Store::<State>::default();
    store.call_hook(State::call_hook);

    fn verify(state: &State) {
        // Calling this func will switch context into wasm, then back to host:
        assert_eq!(state.context, vec![Context::Wasm, Context::Host]);

        assert_eq!(state.calls_into_host, state.returns_from_host + 1);
        assert_eq!(state.calls_into_wasm, state.returns_from_wasm + 1);
    }

    let mut funcs = Vec::new();
    funcs.push(Func::wrap(
        &mut store,
        |caller: Caller<State>, a: i32, b: i64, c: f32, d: f64| {
            verify(caller.data());

            assert_eq!(a, 1);
            assert_eq!(b, 2);
            assert_eq!(c, 3.0);
            assert_eq!(d, 4.0);
        },
    ));
    funcs.push(Func::new(
        &mut store,
        FuncType::new([ValType::I32, ValType::I64, ValType::F32, ValType::F64], []),
        |caller: Caller<State>, params, results| {
            verify(caller.data());

            assert_eq!(params.len(), 4);
            assert_eq!(params[0].i32().unwrap(), 1);
            assert_eq!(params[1].i64().unwrap(), 2);
            assert_eq!(params[2].f32().unwrap(), 3.0);
            assert_eq!(params[3].f64().unwrap(), 4.0);
            assert_eq!(results.len(), 0);
            Ok(())
        },
    ));
    funcs.push(unsafe {
        Func::new_unchecked(
            &mut store,
            FuncType::new([ValType::I32, ValType::I64, ValType::F32, ValType::F64], []),
            |caller: Caller<State>, space| {
                verify(caller.data());

                assert_eq!((*space.add(0)).i32, 1i32.to_le());
                assert_eq!((*space.add(1)).i64, 2i64.to_le());
                assert_eq!((*space.add(2)).f32, 3.0f32.to_bits().to_le());
                assert_eq!((*space.add(3)).f64, 4.0f64.to_bits().to_le());
                Ok(())
            },
        )
    });

    let mut n = 0;
    for f in funcs.iter() {
        f.call(
            &mut store,
            &[Val::I32(1), Val::I64(2), 3.0f32.into(), 4.0f64.into()],
            &mut [],
        )?;
        n += 1;

        // One switch from vm to host to call f, another in return from f.
        assert_eq!(store.data().calls_into_host, n);
        assert_eq!(store.data().returns_from_host, n);
        assert_eq!(store.data().calls_into_wasm, n);
        assert_eq!(store.data().returns_from_wasm, n);

        f.typed::<(i32, i64, f32, f64), (), _>(&store)?
            .call(&mut store, (1, 2, 3.0, 4.0))?;
        n += 1;

        assert_eq!(store.data().calls_into_host, n);
        assert_eq!(store.data().returns_from_host, n);
        assert_eq!(store.data().calls_into_wasm, n);
        assert_eq!(store.data().returns_from_wasm, n);

        unsafe {
            let mut args = [
                Val::I32(1).to_raw(&mut store),
                Val::I64(2).to_raw(&mut store),
                Val::F32(3.0f32.to_bits()).to_raw(&mut store),
                Val::F64(4.0f64.to_bits()).to_raw(&mut store),
            ];
            f.call_unchecked(&mut store, args.as_mut_ptr())?;
        }
        n += 1;

        assert_eq!(store.data().calls_into_host, n);
        assert_eq!(store.data().returns_from_host, n);
        assert_eq!(store.data().calls_into_wasm, n);
        assert_eq!(store.data().returns_from_wasm, n);
    }

    Ok(())
}

// Create an async Func, call it directly:
#[tokio::test]
async fn call_wrapped_async_func() -> Result<(), Error> {
    let mut config = Config::new();
    config.async_support(true);
    let engine = Engine::new(&config)?;
    let mut store = Store::new(&engine, State::default());
    store.call_hook(State::call_hook);
    let f = Func::wrap4_async(
        &mut store,
        |caller: Caller<State>, a: i32, b: i64, c: f32, d: f64| {
            Box::new(async move {
                // Calling this func will switch context into wasm, then back to host:
                assert_eq!(caller.data().context, vec![Context::Wasm, Context::Host]);

                assert_eq!(
                    caller.data().calls_into_host,
                    caller.data().returns_from_host + 1
                );
                assert_eq!(
                    caller.data().calls_into_wasm,
                    caller.data().returns_from_wasm + 1
                );

                assert_eq!(a, 1);
                assert_eq!(b, 2);
                assert_eq!(c, 3.0);
                assert_eq!(d, 4.0);
            })
        },
    );

    f.call_async(
        &mut store,
        &[Val::I32(1), Val::I64(2), 3.0f32.into(), 4.0f64.into()],
        &mut [],
    )
    .await?;

    // One switch from vm to host to call f, another in return from f.
    assert_eq!(store.data().calls_into_host, 1);
    assert_eq!(store.data().returns_from_host, 1);
    assert_eq!(store.data().calls_into_wasm, 1);
    assert_eq!(store.data().returns_from_wasm, 1);

    f.typed::<(i32, i64, f32, f64), (), _>(&store)?
        .call_async(&mut store, (1, 2, 3.0, 4.0))
        .await?;

    assert_eq!(store.data().calls_into_host, 2);
    assert_eq!(store.data().returns_from_host, 2);
    assert_eq!(store.data().calls_into_wasm, 2);
    assert_eq!(store.data().returns_from_wasm, 2);

    Ok(())
}

// Use the Linker to define a sync func, call it through WebAssembly:
#[test]
fn call_linked_func() -> Result<(), Error> {
    let engine = Engine::default();
    let mut store = Store::new(&engine, State::default());
    store.call_hook(State::call_hook);
    let mut linker = Linker::new(&engine);

    linker.func_wrap(
        "host",
        "f",
        |caller: Caller<State>, a: i32, b: i64, c: f32, d: f64| {
            // Calling this func will switch context into wasm, then back to host:
            assert_eq!(caller.data().context, vec![Context::Wasm, Context::Host]);

            assert_eq!(
                caller.data().calls_into_host,
                caller.data().returns_from_host + 1
            );
            assert_eq!(
                caller.data().calls_into_wasm,
                caller.data().returns_from_wasm + 1
            );

            assert_eq!(a, 1);
            assert_eq!(b, 2);
            assert_eq!(c, 3.0);
            assert_eq!(d, 4.0);
        },
    )?;

    let wat = r#"
        (module
            (import "host" "f"
                (func $f (param i32) (param i64) (param f32) (param f64)))
            (func (export "export")
                (call $f (i32.const 1) (i64.const 2) (f32.const 3.0) (f64.const 4.0)))
        )
    "#;
    let module = Module::new(&engine, wat)?;

    let inst = linker.instantiate(&mut store, &module)?;
    let export = inst
        .get_export(&mut store, "export")
        .expect("get export")
        .into_func()
        .expect("export is func");

    export.call(&mut store, &[], &mut [])?;

    // One switch from vm to host to call f, another in return from f.
    assert_eq!(store.data().calls_into_host, 1);
    assert_eq!(store.data().returns_from_host, 1);
    assert_eq!(store.data().calls_into_wasm, 1);
    assert_eq!(store.data().returns_from_wasm, 1);

    export.typed::<(), (), _>(&store)?.call(&mut store, ())?;

    assert_eq!(store.data().calls_into_host, 2);
    assert_eq!(store.data().returns_from_host, 2);
    assert_eq!(store.data().calls_into_wasm, 2);
    assert_eq!(store.data().returns_from_wasm, 2);

    Ok(())
}

// Use the Linker to define an async func, call it through WebAssembly:
#[tokio::test]
async fn call_linked_func_async() -> Result<(), Error> {
    let mut config = Config::new();
    config.async_support(true);
    let engine = Engine::new(&config)?;
    let mut store = Store::new(&engine, State::default());
    store.call_hook(State::call_hook);

    let f = Func::wrap4_async(
        &mut store,
        |caller: Caller<State>, a: i32, b: i64, c: f32, d: f64| {
            Box::new(async move {
                // Calling this func will switch context into wasm, then back to host:
                assert_eq!(caller.data().context, vec![Context::Wasm, Context::Host]);

                assert_eq!(
                    caller.data().calls_into_host,
                    caller.data().returns_from_host + 1
                );
                assert_eq!(
                    caller.data().calls_into_wasm,
                    caller.data().returns_from_wasm + 1
                );
                assert_eq!(a, 1);
                assert_eq!(b, 2);
                assert_eq!(c, 3.0);
                assert_eq!(d, 4.0);
            })
        },
    );

    let mut linker = Linker::new(&engine);

    linker.define("host", "f", f)?;

    let wat = r#"
        (module
            (import "host" "f"
                (func $f (param i32) (param i64) (param f32) (param f64)))
            (func (export "export")
                (call $f (i32.const 1) (i64.const 2) (f32.const 3.0) (f64.const 4.0)))
        )
    "#;
    let module = Module::new(&engine, wat)?;

    let inst = linker.instantiate_async(&mut store, &module).await?;
    let export = inst
        .get_export(&mut store, "export")
        .expect("get export")
        .into_func()
        .expect("export is func");

    export.call_async(&mut store, &[], &mut []).await?;

    // One switch from vm to host to call f, another in return from f.
    assert_eq!(store.data().calls_into_host, 1);
    assert_eq!(store.data().returns_from_host, 1);
    assert_eq!(store.data().calls_into_wasm, 1);
    assert_eq!(store.data().returns_from_wasm, 1);

    export
        .typed::<(), (), _>(&store)?
        .call_async(&mut store, ())
        .await?;

    assert_eq!(store.data().calls_into_host, 2);
    assert_eq!(store.data().returns_from_host, 2);
    assert_eq!(store.data().calls_into_wasm, 2);
    assert_eq!(store.data().returns_from_wasm, 2);

    Ok(())
}

#[test]
fn instantiate() -> Result<(), Error> {
    let mut store = Store::<State>::default();
    store.call_hook(State::call_hook);

    let m = Module::new(store.engine(), "(module)")?;
    Instance::new(&mut store, &m, &[])?;
    assert_eq!(store.data().calls_into_wasm, 0);
    assert_eq!(store.data().calls_into_host, 0);

    let m = Module::new(store.engine(), "(module (func) (start 0))")?;
    Instance::new(&mut store, &m, &[])?;
    assert_eq!(store.data().calls_into_wasm, 1);
    assert_eq!(store.data().calls_into_host, 0);

    Ok(())
}

#[tokio::test]
async fn instantiate_async() -> Result<(), Error> {
    let mut config = Config::new();
    config.async_support(true);
    let engine = Engine::new(&config)?;
    let mut store = Store::new(&engine, State::default());
    store.call_hook(State::call_hook);

    let m = Module::new(store.engine(), "(module)")?;
    Instance::new_async(&mut store, &m, &[]).await?;
    assert_eq!(store.data().calls_into_wasm, 0);
    assert_eq!(store.data().calls_into_host, 0);

    let m = Module::new(store.engine(), "(module (func) (start 0))")?;
    Instance::new_async(&mut store, &m, &[]).await?;
    assert_eq!(store.data().calls_into_wasm, 1);
    assert_eq!(store.data().calls_into_host, 0);

    Ok(())
}

#[test]
fn recursion() -> Result<(), Error> {
    // Make sure call hook behaves reasonably when called recursively

    let engine = Engine::default();
    let mut store = Store::new(&engine, State::default());
    store.call_hook(State::call_hook);
    let mut linker = Linker::new(&engine);

    linker.func_wrap("host", "f", |mut caller: Caller<State>, n: i32| {
        assert_eq!(caller.data().context.last(), Some(&Context::Host));

        assert_eq!(caller.data().calls_into_host, caller.data().calls_into_wasm);

        // Recurse
        if n > 0 {
            caller
                .get_export("export")
                .expect("caller exports \"export\"")
                .into_func()
                .expect("export is a func")
                .typed::<i32, (), _>(&caller)
                .expect("export typing")
                .call(&mut caller, n - 1)
                .unwrap()
        }
    })?;

    let wat = r#"
        (module
            (import "host" "f"
                (func $f (param i32)))
            (func (export "export") (param i32)
                (call $f (local.get 0)))
        )
    "#;
    let module = Module::new(&engine, wat)?;

    let inst = linker.instantiate(&mut store, &module)?;
    let export = inst
        .get_export(&mut store, "export")
        .expect("get export")
        .into_func()
        .expect("export is func");

    // Recursion depth:
    let n: usize = 10;

    export.call(&mut store, &[Val::I32(n as i32)], &mut [])?;

    // Recurse down to 0: n+1 calls
    assert_eq!(store.data().calls_into_host, n + 1);
    assert_eq!(store.data().returns_from_host, n + 1);
    assert_eq!(store.data().calls_into_wasm, n + 1);
    assert_eq!(store.data().returns_from_wasm, n + 1);

    export
        .typed::<i32, (), _>(&store)?
        .call(&mut store, n as i32)?;

    assert_eq!(store.data().calls_into_host, 2 * (n + 1));
    assert_eq!(store.data().returns_from_host, 2 * (n + 1));
    assert_eq!(store.data().calls_into_wasm, 2 * (n + 1));
    assert_eq!(store.data().returns_from_wasm, 2 * (n + 1));

    Ok(())
}

#[test]
fn trapping() -> Result<(), Error> {
    const TRAP_IN_F: i32 = 0;
    const TRAP_NEXT_CALL_HOST: i32 = 1;
    const TRAP_NEXT_RETURN_HOST: i32 = 2;
    const TRAP_NEXT_CALL_WASM: i32 = 3;
    const TRAP_NEXT_RETURN_WASM: i32 = 4;

    let engine = Engine::default();

    let mut linker = Linker::new(&engine);

    linker.func_wrap(
        "host",
        "f",
        |mut caller: Caller<State>, action: i32, recur: i32| -> Result<(), Trap> {
            assert_eq!(caller.data().context.last(), Some(&Context::Host));
            assert_eq!(caller.data().calls_into_host, caller.data().calls_into_wasm);

            match action {
                TRAP_IN_F => return Err(Trap::new("trapping in f")),
                TRAP_NEXT_CALL_HOST => caller.data_mut().trap_next_call_host = true,
                TRAP_NEXT_RETURN_HOST => caller.data_mut().trap_next_return_host = true,
                TRAP_NEXT_CALL_WASM => caller.data_mut().trap_next_call_wasm = true,
                TRAP_NEXT_RETURN_WASM => caller.data_mut().trap_next_return_wasm = true,
                _ => {} // Do nothing
            }

            // recur so that we can trigger a next call.
            // propogate its trap, if it traps!
            if recur > 0 {
                let _ = caller
                    .get_export("export")
                    .expect("caller exports \"export\"")
                    .into_func()
                    .expect("export is a func")
                    .typed::<(i32, i32), (), _>(&caller)
                    .expect("export typing")
                    .call(&mut caller, (action, 0))?;
            }

            Ok(())
        },
    )?;

    let wat = r#"
        (module
            (import "host" "f"
                (func $f (param i32) (param i32)))
            (func (export "export") (param i32) (param i32)
                (call $f (local.get 0) (local.get 1)))
        )
    "#;
    let module = Module::new(&engine, wat)?;

    let run = |action: i32, recur: bool| -> (State, Option<Error>) {
        let mut store = Store::new(&engine, State::default());
        store.call_hook(State::call_hook);
        let inst = linker
            .instantiate(&mut store, &module)
            .expect("instantiate");
        let export = inst
            .get_export(&mut store, "export")
            .expect("get export")
            .into_func()
            .expect("export is func");

        let r = export.call(
            &mut store,
            &[Val::I32(action), Val::I32(if recur { 1 } else { 0 })],
            &mut [],
        );
        (store.into_data(), r.err())
    };

    let (s, e) = run(TRAP_IN_F, false);
    assert!(e.unwrap().to_string().starts_with("trapping in f"));
    assert_eq!(s.calls_into_host, 1);
    assert_eq!(s.returns_from_host, 1);
    assert_eq!(s.calls_into_wasm, 1);
    assert_eq!(s.returns_from_wasm, 1);

    // trap in next call to host. No calls after the bit is set, so this trap shouldn't happen
    let (s, e) = run(TRAP_NEXT_CALL_HOST, false);
    assert!(e.is_none());
    assert_eq!(s.calls_into_host, 1);
    assert_eq!(s.returns_from_host, 1);
    assert_eq!(s.calls_into_wasm, 1);
    assert_eq!(s.returns_from_wasm, 1);

    // trap in next call to host. recur, so the second call into host traps:
    let (s, e) = run(TRAP_NEXT_CALL_HOST, true);
    assert!(e
        .unwrap()
        .to_string()
        .starts_with("call_hook: trapping on CallingHost"));
    assert_eq!(s.calls_into_host, 2);
    assert_eq!(s.returns_from_host, 1);
    assert_eq!(s.calls_into_wasm, 2);
    assert_eq!(s.returns_from_wasm, 2);

    // trap in the return from host. should trap right away, without recursion
    let (s, e) = run(TRAP_NEXT_RETURN_HOST, false);
    assert!(e
        .unwrap()
        .to_string()
        .starts_with("call_hook: trapping on ReturningFromHost"));
    assert_eq!(s.calls_into_host, 1);
    assert_eq!(s.returns_from_host, 1);
    assert_eq!(s.calls_into_wasm, 1);
    assert_eq!(s.returns_from_wasm, 1);

    // trap in next call to wasm. No calls after the bit is set, so this trap shouldnt happen:
    let (s, e) = run(TRAP_NEXT_CALL_WASM, false);
    assert!(e.is_none());
    assert_eq!(s.calls_into_host, 1);
    assert_eq!(s.returns_from_host, 1);
    assert_eq!(s.calls_into_wasm, 1);
    assert_eq!(s.returns_from_wasm, 1);

    // trap in next call to wasm. recur, so the second call into wasm traps:
    let (s, e) = run(TRAP_NEXT_CALL_WASM, true);
    assert!(e
        .unwrap()
        .to_string()
        .starts_with("call_hook: trapping on CallingWasm"));
    assert_eq!(s.calls_into_host, 1);
    assert_eq!(s.returns_from_host, 1);
    assert_eq!(s.calls_into_wasm, 2);
    assert_eq!(s.returns_from_wasm, 1);

    // trap in the return from wasm. should trap right away, without recursion
    let (s, e) = run(TRAP_NEXT_RETURN_WASM, false);
    assert!(e
        .unwrap()
        .to_string()
        .starts_with("call_hook: trapping on ReturningFromWasm"));
    assert_eq!(s.calls_into_host, 1);
    assert_eq!(s.returns_from_host, 1);
    assert_eq!(s.calls_into_wasm, 1);
    assert_eq!(s.returns_from_wasm, 1);

    Ok(())
}

#[tokio::test]
async fn basic_async_hook() -> Result<(), Error> {
    struct HandlerR;

    #[async_trait::async_trait]
    impl CallHookHandler<State> for HandlerR {
        async fn handle_call_event(
            &self,
            obj: &mut State,
            ch: CallHook,
        ) -> Result<(), wasmtime::Trap> {
            State::call_hook(obj, ch)
        }
    }
    let mut config = Config::new();
    config.async_support(true);
    let engine = Engine::new(&config)?;
    let mut store = Store::new(&engine, State::default());
    store.call_hook_async(HandlerR {});

    assert_eq!(store.data().calls_into_host, 0);
    assert_eq!(store.data().returns_from_host, 0);
    assert_eq!(store.data().calls_into_wasm, 0);
    assert_eq!(store.data().returns_from_wasm, 0);

    let mut linker = Linker::new(&engine);

    linker.func_wrap(
        "host",
        "f",
        |caller: Caller<State>, a: i32, b: i64, c: f32, d: f64| {
            // Calling this func will switch context into wasm, then back to host:
            assert_eq!(caller.data().context, vec![Context::Wasm, Context::Host]);

            assert_eq!(
                caller.data().calls_into_host,
                caller.data().returns_from_host + 1
            );
            assert_eq!(
                caller.data().calls_into_wasm,
                caller.data().returns_from_wasm + 1
            );

            assert_eq!(a, 1);
            assert_eq!(b, 2);
            assert_eq!(c, 3.0);
            assert_eq!(d, 4.0);
        },
    )?;

    let wat = r#"
        (module
            (import "host" "f"
                (func $f (param i32) (param i64) (param f32) (param f64)))
            (func (export "export")
                (call $f (i32.const 1) (i64.const 2) (f32.const 3.0) (f64.const 4.0)))
        )
    "#;
    let module = Module::new(&engine, wat)?;

    let inst = linker.instantiate_async(&mut store, &module).await?;
    let export = inst
        .get_export(&mut store, "export")
        .expect("get export")
        .into_func()
        .expect("export is func");

    export.call_async(&mut store, &[], &mut []).await?;

    // One switch from vm to host to call f, another in return from f.
    assert_eq!(store.data().calls_into_host, 1);
    assert_eq!(store.data().returns_from_host, 1);
    assert_eq!(store.data().calls_into_wasm, 1);
    assert_eq!(store.data().returns_from_wasm, 1);

    Ok(())
}

#[tokio::test]
async fn timeout_async_hook() -> Result<(), Error> {
    struct HandlerR;

    #[async_trait::async_trait]
    impl CallHookHandler<State> for HandlerR {
        async fn handle_call_event(
            &self,
            obj: &mut State,
            ch: CallHook,
        ) -> Result<(), wasmtime::Trap> {
            if obj.calls_into_host > 200 {
                return Err(wasmtime::Trap::new("timeout"));
            }

            match ch {
                CallHook::CallingHost => obj.calls_into_host += 1,
                CallHook::CallingWasm => obj.calls_into_wasm += 1,
                CallHook::ReturningFromHost => obj.returns_from_host += 1,
                CallHook::ReturningFromWasm => obj.returns_from_wasm += 1,
            }

            Ok(())
        }
    }

    let mut config = Config::new();
    config.async_support(true);
    let engine = Engine::new(&config)?;
    let mut store = Store::new(&engine, State::default());
    store.call_hook_async(HandlerR {});

    assert_eq!(store.data().calls_into_host, 0);
    assert_eq!(store.data().returns_from_host, 0);
    assert_eq!(store.data().calls_into_wasm, 0);
    assert_eq!(store.data().returns_from_wasm, 0);

    let mut linker = Linker::new(&engine);

    linker.func_wrap(
        "host",
        "f",
        |_caller: Caller<State>, a: i32, b: i64, c: f32, d: f64| {
            assert_eq!(a, 1);
            assert_eq!(b, 2);
            assert_eq!(c, 3.0);
            assert_eq!(d, 4.0);
        },
    )?;

    let wat = r#"
        (module
            (import "host" "f"
                (func $f (param i32) (param i64) (param f32) (param f64)))
            (func (export "export")
                (loop $start
                    (call $f (i32.const 1) (i64.const 2) (f32.const 3.0) (f64.const 4.0))
                    (br $start)))
        )
    "#;
    let module = Module::new(&engine, wat)?;

    let inst = linker.instantiate_async(&mut store, &module).await?;
    let export = inst
        .get_typed_func::<(), (), _>(&mut store, "export")
        .expect("export is func");

    store.set_epoch_deadline(1);
    store.epoch_deadline_async_yield_and_update(1);
    assert!(export.call_async(&mut store, ()).await.is_err());

    // One switch from vm to host to call f, another in return from f.
    assert!(store.data().calls_into_host > 1);
    assert!(store.data().returns_from_host > 1);
    assert_eq!(store.data().calls_into_wasm, 1);
    assert_eq!(store.data().returns_from_wasm, 0);

    Ok(())
}

#[tokio::test]
async fn drop_suspended_async_hook() -> Result<(), Error> {
    struct Handler;

    #[async_trait::async_trait]
    impl CallHookHandler<u32> for Handler {
        async fn handle_call_event(
            &self,
            state: &mut u32,
            _ch: CallHook,
        ) -> Result<(), wasmtime::Trap> {
            assert_eq!(*state, 0);
            *state += 1;
            let _dec = Decrement(state);

            // Simulate some sort of event which takes a number of yields
            for _ in 0..500 {
                tokio::task::yield_now().await;
            }
            Ok(())
        }
    }

    let mut config = Config::new();
    config.async_support(true);
    let engine = Engine::new(&config)?;
    let mut store = Store::new(&engine, 0);
    store.call_hook_async(Handler);

    let mut linker = Linker::new(&engine);

    // Simulate a host function that has lots of yields with an infinite loop.
    linker.func_wrap0_async("host", "f", |mut cx| {
        Box::new(async move {
            let state = cx.data_mut();
            assert_eq!(*state, 0);
            *state += 1;
            let _dec = Decrement(state);
            loop {
                tokio::task::yield_now().await;
            }
        })
    })?;

    let wat = r#"
        (module
            (import "host" "f" (func $f))
            (func (export "") call $f)
        )
    "#;
    let module = Module::new(&engine, wat)?;

    let inst = linker.instantiate_async(&mut store, &module).await?;
    assert_eq!(*store.data(), 0);
    let export = inst
        .get_typed_func::<(), (), _>(&mut store, "")
        .expect("export is func");

    // First test that if we drop in the middle of an async hook that everything
    // is alright.
    PollNTimes {
        future: Box::pin(export.call_async(&mut store, ())),
        times: 200,
    }
    .await;
    assert_eq!(*store.data(), 0); // double-check user dtors ran

    // Next test that if we drop while in a host async function that everything
    // is also alright.
    PollNTimes {
        future: Box::pin(export.call_async(&mut store, ())),
        times: 1_000,
    }
    .await;
    assert_eq!(*store.data(), 0); // double-check user dtors ran

    return Ok(());

    // A helper struct to poll an inner `future` N `times` and then resolve.
    // This is used above to test that when futures are dropped while they're
    // pending everything works and is cleaned up on the Wasmtime side of
    // things.
    struct PollNTimes<F> {
        future: F,
        times: u32,
    }

    impl<F: Future + Unpin> Future for PollNTimes<F> {
        type Output = ();
        fn poll(mut self: Pin<&mut Self>, task: &mut task::Context<'_>) -> Poll<()> {
            for _ in 0..self.times {
                match Pin::new(&mut self.future).poll(task) {
                    Poll::Ready(_) => panic!("future should not be ready"),
                    Poll::Pending => {}
                }
            }

            Poll::Ready(())
        }
    }

    // helper struct to decrement a counter on drop
    struct Decrement<'a>(&'a mut u32);

    impl Drop for Decrement<'_> {
        fn drop(&mut self) {
            *self.0 -= 1;
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Context {
    Host,
    Wasm,
}

struct State {
    context: Vec<Context>,

    calls_into_host: usize,
    returns_from_host: usize,
    calls_into_wasm: usize,
    returns_from_wasm: usize,

    trap_next_call_host: bool,
    trap_next_return_host: bool,
    trap_next_call_wasm: bool,
    trap_next_return_wasm: bool,
}

impl Default for State {
    fn default() -> Self {
        State {
            context: Vec::new(),
            calls_into_host: 0,
            returns_from_host: 0,
            calls_into_wasm: 0,
            returns_from_wasm: 0,
            trap_next_call_host: false,
            trap_next_return_host: false,
            trap_next_call_wasm: false,
            trap_next_return_wasm: false,
        }
    }
}

impl State {
    // This implementation asserts that hooks are always called in a stack-like manner.
    fn call_hook(&mut self, s: CallHook) -> Result<(), Trap> {
        match s {
            CallHook::CallingHost => {
                self.calls_into_host += 1;
                if self.trap_next_call_host {
                    return Err(Trap::new("call_hook: trapping on CallingHost"));
                } else {
                    self.context.push(Context::Host);
                }
            }
            CallHook::ReturningFromHost => match self.context.pop() {
                Some(Context::Host) => {
                    self.returns_from_host += 1;
                    if self.trap_next_return_host {
                        return Err(Trap::new("call_hook: trapping on ReturningFromHost"));
                    }
                }
                c => panic!(
                    "illegal context: expected Some(Host), got {:?}. remaining: {:?}",
                    c, self.context
                ),
            },
            CallHook::CallingWasm => {
                self.calls_into_wasm += 1;
                if self.trap_next_call_wasm {
                    return Err(Trap::new("call_hook: trapping on CallingWasm"));
                } else {
                    self.context.push(Context::Wasm);
                }
            }
            CallHook::ReturningFromWasm => match self.context.pop() {
                Some(Context::Wasm) => {
                    self.returns_from_wasm += 1;
                    if self.trap_next_return_wasm {
                        return Err(Trap::new("call_hook: trapping on ReturningFromWasm"));
                    }
                }
                c => panic!(
                    "illegal context: expected Some(Wasm), got {:?}. remaining: {:?}",
                    c, self.context
                ),
            },
        }
        Ok(())
    }
}
