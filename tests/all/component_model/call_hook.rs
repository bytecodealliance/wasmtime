#![cfg(not(miri))]

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
async fn basic_async_hook() -> Result<()> {
    struct HandlerR;

    #[async_trait::async_trait]
    impl CallHookHandler<State> for HandlerR {
        async fn handle_call_event(
            &self,
            ctx: StoreContextMut<'_, State>,
            ch: CallHook,
        ) -> Result<()> {
            sync_call_hook(ctx, ch)
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

    export.call_async(&mut store, ()).await?;
    export.post_return_async(&mut store).await?;

    let s = store.into_data();
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
    fn call_hook(&mut self, s: CallHook) -> Result<()> {
        match s {
            CallHook::CallingHost => {
                self.calls_into_host += 1;
                if self.trap_next_call_host {
                    bail!("call_hook: trapping on CallingHost");
                } else {
                    self.context.push(Context::Host);
                }
            }
            CallHook::ReturningFromHost => match self.context.pop() {
                Some(Context::Host) => {
                    self.returns_from_host += 1;
                    if self.trap_next_return_host {
                        bail!("call_hook: trapping on ReturningFromHost");
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
                    bail!("call_hook: trapping on CallingWasm");
                } else {
                    self.context.push(Context::Wasm);
                }
            }
            CallHook::ReturningFromWasm => match self.context.pop() {
                Some(Context::Wasm) => {
                    self.returns_from_wasm += 1;
                    if self.trap_next_return_wasm {
                        bail!("call_hook: trapping on ReturningFromWasm");
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

fn sync_call_hook(mut ctx: StoreContextMut<'_, State>, transition: CallHook) -> Result<()> {
    ctx.data_mut().call_hook(transition)
}
