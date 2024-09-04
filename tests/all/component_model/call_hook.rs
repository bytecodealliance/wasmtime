#![cfg(not(miri))]

use super::REALLOC_AND_FREE;
use crate::call_hook::{sync_call_hook, Context, State};
use anyhow::{bail, Result};
use std::future::Future;
use std::pin::Pin;
use std::task::{self, Poll};
use wasmtime::component::*;
use wasmtime::{CallHook, CallHookHandler, Config, Engine, Store, StoreContextMut};

// Crate a synchronous Func, call it directly:
#[test]
fn call_wrapped_func() -> Result<()> {
    let wat = r#"
        (component
            (import "f" (func $f))

            (core func $f_lower
                (canon lower (func $f))
            )
            (core module $m
                (import "" "" (func $f))

                (func $export
                    (call $f)
                )

                (export "export" (func $export))
            )
            (core instance $i (instantiate $m
                (with "" (instance
                    (export "" (func $f_lower))
                ))
            ))
            (func (export "export")
                (canon lift
                    (core func $i "export")
                )
            )
        )
    "#;

    let engine = Engine::default();
    let component = Component::new(&engine, wat)?;

    let mut linker = Linker::<State>::new(&engine);
    linker
        .root()
        .func_wrap("f", |_, _: ()| -> Result<()> { Ok(()) })?;

    let mut store = Store::new(&engine, State::default());
    store.call_hook(sync_call_hook);
    let inst = linker
        .instantiate(&mut store, &component)
        .expect("instantiate");

    let export = inst
        .get_typed_func::<(), ()>(&mut store, "export")
        .expect("looking up `export`");

    export.call(&mut store, ())?;
    export.post_return(&mut store)?;

    let s = store.into_data();
    assert_eq!(s.calls_into_host, 1);
    assert_eq!(s.returns_from_host, 1);
    assert_eq!(s.calls_into_wasm, 1);
    assert_eq!(s.returns_from_wasm, 1);

    Ok(())
}

// Call a func that turns a `list<u8>` into a `string`, to ensure that `realloc` calls are counted.
#[test]
fn call_func_with_realloc() -> Result<()> {
    let wat = format!(
        r#"(component
            (core module $m
                (memory (export "memory") 1)
                (func (export "roundtrip") (param i32 i32) (result i32)
                    (local $base i32)
                    (local.set $base
                        (call $realloc
                            (i32.const 0)
                            (i32.const 0)
                            (i32.const 4)
                            (i32.const 8)))
                    (i32.store offset=0
                        (local.get $base)
                        (local.get 0))
                    (i32.store offset=4
                        (local.get $base)
                        (local.get 1))
                    (local.get $base)
                )

                {REALLOC_AND_FREE}
            )
            (core instance $i (instantiate $m))

            (func (export "list8-to-str") (param "a" (list u8)) (result string)
                (canon lift
                    (core func $i "roundtrip")
                    (memory $i "memory")
                    (realloc (func $i "realloc"))
                )
            )
        )"#
    );

    let engine = Engine::default();
    let component = Component::new(&engine, wat)?;
    let linker = Linker::<State>::new(&engine);
    let mut store = Store::new(&engine, State::default());
    store.call_hook(sync_call_hook);
    let inst = linker
        .instantiate(&mut store, &component)
        .expect("instantiate");

    let export = inst
        .get_typed_func::<(&[u8],), (WasmStr,)>(&mut store, "list8-to-str")
        .expect("looking up `list8-to-str`");

    let message = String::from("hello, world!");
    let res = export.call(&mut store, (message.as_bytes(),))?.0;
    let result = res.to_str(&store)?;
    assert_eq!(&message, &result);

    assert_eq!(store.data().calls_into_wasm, 2);
    assert_eq!(store.data().returns_from_wasm, 2);

    export.post_return(&mut store)?;

    // There is one host call for the host-side realloc, and then two wasm calls for both the
    // `list8-to-str` call and the guest realloc call for the list argument.
    let s = store.into_data();
    assert_eq!(s.calls_into_host, 1);
    assert_eq!(s.returns_from_host, 1);
    assert_eq!(s.calls_into_wasm, 2);
    assert_eq!(s.returns_from_wasm, 2);

    Ok(())
}

// Call a guest function that also defines a post-return.
#[test]
fn call_func_with_post_return() -> Result<()> {
    let wat =
        r#"(component
            (core module $m
                (func (export "roundtrip"))
                (func (export "post-return"))
            )
            (core instance $i (instantiate $m))

            (func (export "export")
                (canon lift
                    (core func $i "roundtrip")
                    (post-return (func $i "post-return"))
                )
            )
        )"#;

    let engine = Engine::default();
    let component = Component::new(&engine, wat)?;
    let linker = Linker::<State>::new(&engine);
    let mut store = Store::new(&engine, State::default());
    store.call_hook(sync_call_hook);
    let inst = linker
        .instantiate(&mut store, &component)
        .expect("instantiate");

    let export = inst
        .get_typed_func::<(), ()>(&mut store, "export")
        .expect("looking up `export`");

    export.call(&mut store, ())?;

    // Before post-return, there will only have been one call into wasm.
    assert_eq!(store.data().calls_into_wasm, 1);
    assert_eq!(store.data().returns_from_wasm, 1);

    export.post_return(&mut store)?;

    // There are no host calls in this example, but the post-return does increment the count of
    // wasm calls by 1, putting the total number of wasm calls at 2.
    let s = store.into_data();
    assert_eq!(s.calls_into_host, 0);
    assert_eq!(s.returns_from_host, 0);
    assert_eq!(s.calls_into_wasm, 2);
    assert_eq!(s.returns_from_wasm, 2);

    Ok(())
}

// Create an async Func, call it directly:
#[tokio::test]
async fn call_wrapped_async_func() -> Result<()> {
    let wat = r#"
        (component
            (import "f" (func $f))

            (core func $f_lower
                (canon lower (func $f))
            )
            (core module $m
                (import "" "" (func $f))

                (func $export
                    (call $f)
                )

                (export "export" (func $export))
            )
            (core instance $i (instantiate $m
                (with "" (instance
                    (export "" (func $f_lower))
                ))
            ))
            (func (export "export")
                (canon lift
                    (core func $i "export")
                )
            )
        )
    "#;

    let mut config = Config::new();
    config.async_support(true);
    let engine = Engine::new(&config)?;

    let component = Component::new(&engine, wat)?;

    let mut linker = Linker::<State>::new(&engine);
    linker
        .root()
        .func_wrap_async("f", |_, _: ()| Box::new(async { Ok(()) }))?;

    let mut store = Store::new(&engine, State::default());
    store.call_hook(sync_call_hook);

    let inst = linker
        .instantiate_async(&mut store, &component)
        .await
        .expect("instantiate");

    let export = inst
        .get_typed_func::<(), ()>(&mut store, "export")
        .expect("looking up `export`");

    export.call_async(&mut store, ()).await?;
    export.post_return_async(&mut store).await?;

    let s = store.into_data();
    assert_eq!(s.calls_into_host, 1);
    assert_eq!(s.returns_from_host, 1);
    assert_eq!(s.calls_into_wasm, 1);
    assert_eq!(s.returns_from_wasm, 1);

    Ok(())
}

#[test]
fn trapping() -> Result<()> {
    const TRAP_IN_F: i32 = 0;
    const TRAP_NEXT_CALL_HOST: i32 = 1;
    const TRAP_NEXT_RETURN_HOST: i32 = 2;
    const TRAP_NEXT_CALL_WASM: i32 = 3;
    const TRAP_NEXT_RETURN_WASM: i32 = 4;
    const DO_NOTHING: i32 = 5;

    let engine = Engine::default();

    let mut linker = Linker::<State>::new(&engine);

    linker
        .root()
        .func_wrap("f", |mut store: _, (action,): (i32,)| -> Result<()> {
            assert_eq!(store.data().context.last(), Some(&Context::Host));
            assert_eq!(store.data().calls_into_host, store.data().calls_into_wasm);

            match action {
                TRAP_IN_F => bail!("trapping in f"),
                TRAP_NEXT_CALL_HOST => store.data_mut().trap_next_call_host = true,
                TRAP_NEXT_RETURN_HOST => store.data_mut().trap_next_return_host = true,
                TRAP_NEXT_CALL_WASM => store.data_mut().trap_next_call_wasm = true,
                TRAP_NEXT_RETURN_WASM => store.data_mut().trap_next_return_wasm = true,
                _ => {} // Do nothing
            }

            Ok(())
        })?;

    let wat = r#"
        (component
            (import "f" (func $f (param "action" s32)))

            (core func $f_lower
                (canon lower (func $f))
            )
            (core module $m
                (import "" "" (func $f (param i32)))

                (func $export (param i32)
                    (call $f (local.get 0))
                )

                (export "export" (func $export))
            )
            (core instance $i (instantiate $m
                (with "" (instance
                    (export "" (func $f_lower))
                ))
            ))
            (func (export "export") (param "action" s32)
                (canon lift
                    (core func $i "export")
                )
            )
        )
    "#;

    let component = Component::new(&engine, wat)?;

    let run = |action: i32, again: bool| -> (State, Option<anyhow::Error>) {
        let mut store = Store::new(&engine, State::default());
        store.call_hook(sync_call_hook);
        let inst = linker
            .instantiate(&mut store, &component)
            .expect("instantiate");

        let export = inst
            .get_typed_func::<(i32,), ()>(&mut store, "export")
            .expect("looking up `export`");

        let mut r = export.call(&mut store, (action,));
        if r.is_ok() && again {
            export.post_return(&mut store).unwrap();
            r = export.call(&mut store, (action,));
        }
        (store.into_data(), r.err())
    };

    let (s, e) = run(DO_NOTHING, false);
    assert!(e.is_none());
    assert_eq!(s.calls_into_host, 1);
    assert_eq!(s.returns_from_host, 1);
    assert_eq!(s.calls_into_wasm, 1);
    assert_eq!(s.returns_from_wasm, 1);

    let (s, e) = run(DO_NOTHING, true);
    assert!(e.is_none());
    assert_eq!(s.calls_into_host, 2);
    assert_eq!(s.returns_from_host, 2);
    assert_eq!(s.calls_into_wasm, 2);
    assert_eq!(s.returns_from_wasm, 2);

    let (s, e) = run(TRAP_IN_F, false);
    assert!(format!("{:?}", e.unwrap()).contains("trapping in f"));
    assert_eq!(s.calls_into_host, 1);
    assert_eq!(s.returns_from_host, 1);
    assert_eq!(s.calls_into_wasm, 1);
    assert_eq!(s.returns_from_wasm, 1);

    // // trap in next call to host. No calls after the bit is set, so this trap shouldn't happen
    let (s, e) = run(TRAP_NEXT_CALL_HOST, false);
    assert!(e.is_none());
    assert_eq!(s.calls_into_host, 1);
    assert_eq!(s.returns_from_host, 1);
    assert_eq!(s.calls_into_wasm, 1);
    assert_eq!(s.returns_from_wasm, 1);

    // trap in next call to host. call again, so the second call into host traps:
    let (s, e) = run(TRAP_NEXT_CALL_HOST, true);
    println!("{:?}", e.as_ref().unwrap());
    assert!(format!("{:?}", e.unwrap()).contains("call_hook: trapping on CallingHost"));
    assert_eq!(s.calls_into_host, 2);
    assert_eq!(s.returns_from_host, 1);
    assert_eq!(s.calls_into_wasm, 2);
    assert_eq!(s.returns_from_wasm, 2);

    // trap in the return from host. should trap right away, without a second call
    let (s, e) = run(TRAP_NEXT_RETURN_HOST, false);
    assert!(format!("{:?}", e.unwrap()).contains("call_hook: trapping on ReturningFromHost"));
    assert_eq!(s.calls_into_host, 1);
    assert_eq!(s.returns_from_host, 1);
    assert_eq!(s.calls_into_wasm, 1);
    assert_eq!(s.returns_from_wasm, 1);

    // trap in next call to wasm. No calls after the bit is set, so this trap shouldn't happen:
    let (s, e) = run(TRAP_NEXT_CALL_WASM, false);
    assert!(e.is_none());
    assert_eq!(s.calls_into_host, 1);
    assert_eq!(s.returns_from_host, 1);
    assert_eq!(s.calls_into_wasm, 1);
    assert_eq!(s.returns_from_wasm, 1);

    // trap in next call to wasm. call again, so the second call into wasm traps:
    let (s, e) = run(TRAP_NEXT_CALL_WASM, true);
    assert!(format!("{:?}", e.unwrap()).contains("call_hook: trapping on CallingWasm"));
    assert_eq!(s.calls_into_host, 1);
    assert_eq!(s.returns_from_host, 1);
    assert_eq!(s.calls_into_wasm, 2);
    assert_eq!(s.returns_from_wasm, 1);

    // trap in the return from wasm. should trap right away, without a second call
    let (s, e) = run(TRAP_NEXT_RETURN_WASM, false);
    assert!(format!("{:?}", e.unwrap()).contains("call_hook: trapping on ReturningFromWasm"));
    assert_eq!(s.calls_into_host, 1);
    assert_eq!(s.returns_from_host, 1);
    assert_eq!(s.calls_into_wasm, 1);
    assert_eq!(s.returns_from_wasm, 1);

    Ok(())
}

#[tokio::test]
async fn timeout_async_hook() -> Result<()> {
    struct HandlerR;

    #[async_trait::async_trait]
    impl CallHookHandler<State> for HandlerR {
        async fn handle_call_event(
            &self,
            mut ctx: StoreContextMut<'_, State>,
            ch: CallHook,
        ) -> Result<()> {
            let obj = ctx.data_mut();
            if obj.calls_into_host > 200 {
                bail!("timeout");
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

    let wat = r#"
        (component
            (import "f" (func $f))

            (core func $f_lower
                (canon lower (func $f))
            )
            (core module $m
                (import "" "" (func $f))

                (func $export
                    (loop $start
                        (call $f)
                        (br $start))
                )

                (export "export" (func $export))
            )
            (core instance $i (instantiate $m
                (with "" (instance
                    (export "" (func $f_lower))
                ))
            ))
            (func (export "export")
                (canon lift
                    (core func $i "export")
                )
            )
        )
    "#;

    let mut config = Config::new();
    config.async_support(true);
    let engine = Engine::new(&config)?;

    let component = Component::new(&engine, wat)?;

    let mut linker = Linker::<State>::new(&engine);
    linker
        .root()
        .func_wrap_async("f", |_, _: ()| Box::new(async { Ok(()) }))?;

    let mut store = Store::new(&engine, State::default());
    store.call_hook_async(HandlerR {});

    let inst = linker
        .instantiate_async(&mut store, &component)
        .await
        .expect("instantiate");

    let export = inst
        .get_typed_func::<(), ()>(&mut store, "export")
        .expect("looking up `export`");

    let r = export.call_async(&mut store, ()).await;
    assert!(format!("{:?}", r.unwrap_err()).contains("timeout"));

    let s = store.into_data();
    assert!(s.calls_into_host > 1);
    assert!(s.returns_from_host > 1);
    assert_eq!(s.calls_into_wasm, 1);
    assert_eq!(s.returns_from_wasm, 0);

    Ok(())
}

#[tokio::test]
async fn drop_suspended_async_hook() -> Result<()> {
    struct Handler;

    #[async_trait::async_trait]
    impl CallHookHandler<u32> for Handler {
        async fn handle_call_event(
            &self,
            mut ctx: StoreContextMut<'_, u32>,
            _ch: CallHook,
        ) -> Result<()> {
            let state = ctx.data_mut();
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

    let wat = r#"
        (component
            (import "f" (func $f))

            (core func $f_lower
                (canon lower (func $f))
            )
            (core module $m
                (import "" "" (func $f))

                (func $export
                    (call $f)
                )

                (export "export" (func $export))
            )
            (core instance $i (instantiate $m
                (with "" (instance
                    (export "" (func $f_lower))
                ))
            ))
            (func (export "export")
                (canon lift
                    (core func $i "export")
                )
            )
        )
    "#;

    let mut config = Config::new();
    config.async_support(true);
    let engine = Engine::new(&config)?;

    let component = Component::new(&engine, wat)?;

    let mut linker = Linker::<u32>::new(&engine);
    linker.root().func_wrap_async("f", |mut store, _: ()| {
        Box::new(async move {
            let state = store.data_mut();
            assert_eq!(*state, 0);
            *state += 1;
            let _dec = Decrement(state);
            for _ in 0.. {
                tokio::task::yield_now().await;
            }
            Ok(())
        })
    })?;

    let mut store = Store::new(&engine, 0);
    store.call_hook_async(Handler);

    let inst = linker
        .instantiate_async(&mut store, &component)
        .await
        .expect("instantiate");

    let export = inst
        .get_typed_func::<(), ()>(&mut store, "export")
        .expect("looking up `export`");

    // Test that if we drop in the middle of an async hook that everything
    // is alright.
    PollNTimes {
        future: Box::pin(export.call_async(&mut store, ())),
        times: 200,
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
            for i in 0..self.times {
                match Pin::new(&mut self.future).poll(task) {
                    Poll::Ready(_) => panic!("future should not be ready at {i}"),
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
