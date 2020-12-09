use std::cell::Cell;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};
use wasmtime::*;

fn async_store() -> Store {
    let engine = Engine::default();
    Store::new_async(&engine)
}

#[test]
fn smoke() {
    let store = async_store();
    let func = Func::new_async(
        &store,
        FuncType::new(None, None),
        (),
        move |_caller, _state, _params, _results| Box::new(async { Ok(()) }),
    );
    run(func.call_async(&[])).unwrap();
    run(func.call_async(&[])).unwrap();
    let future1 = func.call_async(&[]);
    let future2 = func.call_async(&[]);
    run(future2).unwrap();
    run(future1).unwrap();
}

#[test]
fn smoke_with_suspension() {
    let store = async_store();
    let func = Func::new_async(
        &store,
        FuncType::new(None, None),
        (),
        move |_caller, _state, _params, _results| {
            Box::new(async {
                PendingOnce::default().await;
                Ok(())
            })
        },
    );
    run(func.call_async(&[])).unwrap();
    run(func.call_async(&[])).unwrap();
    let future1 = func.call_async(&[]);
    let future2 = func.call_async(&[]);
    run(future2).unwrap();
    run(future1).unwrap();
}

#[test]
fn smoke_get_with_suspension() {
    let store = async_store();
    let func = Func::new_async(
        &store,
        FuncType::new(None, None),
        (),
        move |_caller, _state, _params, _results| {
            Box::new(async {
                PendingOnce::default().await;
                Ok(())
            })
        },
    );
    let func = func.get0_async::<()>().unwrap();
    run(func()).unwrap();
    run(func()).unwrap();
    let future1 = func();
    let future2 = func();
    run(future2).unwrap();
    run(future1).unwrap();
}

#[test]
fn recursive_call() {
    let store = async_store();
    let async_wasm_func = Rc::new(Func::new_async(
        &store,
        FuncType::new(None, None),
        (),
        |_caller, _state, _params, _results| {
            Box::new(async {
                PendingOnce::default().await;
                Ok(())
            })
        },
    ));
    let weak = Rc::downgrade(&async_wasm_func);

    // Create an imported function which recursively invokes another wasm
    // function asynchronously, although this one is just our own host function
    // which suffices for this test.
    let func2 = Func::new_async(
        &store,
        FuncType::new(None, None),
        (),
        move |_caller, _state, _params, _results| {
            let async_wasm_func = weak.upgrade().unwrap();
            Box::new(async move {
                async_wasm_func.call_async(&[]).await?;
                Ok(())
            })
        },
    );

    // Create an instance which calls an async import twice.
    let module = Module::new(
        store.engine(),
        "
            (module
                (import \"\" \"\" (func))
                (func (export \"\")
                    ;; call imported function which recursively does an async
                    ;; call
                    call 0
                    ;; do it again, and our various pointers all better align
                    call 0))
        ",
    )
    .unwrap();

    run(async {
        let instance = Instance::new_async(&store, &module, &[func2.into()]).await?;
        let func = instance.get_func("").unwrap();
        func.call_async(&[]).await
    })
    .unwrap();
}

#[test]
fn suspend_while_suspending() {
    let store = async_store();

    // Create a synchronous function which calls our asynchronous function and
    // runs it locally. This shouldn't generally happen but we know everything
    // is synchronous in this test so it's fine for us to do this.
    //
    // The purpose of this test is intended to stress various cases in how
    // we manage pointers in ways that are not necessarily common but are still
    // possible in safe code.
    let async_thunk = Rc::new(Func::new_async(
        &store,
        FuncType::new(None, None),
        (),
        |_caller, _state, _params, _results| Box::new(async { Ok(()) }),
    ));
    let weak = Rc::downgrade(&async_thunk);
    let sync_call_async_thunk = Func::new(
        &store,
        FuncType::new(None, None),
        move |_caller, _params, _results| {
            let async_thunk = weak.upgrade().unwrap();
            run(async_thunk.call_async(&[]))?;
            Ok(())
        },
    );

    // A small async function that simply awaits once to pump the loops and
    // then finishes.
    let async_import = Func::new_async(
        &store,
        FuncType::new(None, None),
        (),
        move |_caller, _state, _params, _results| {
            Box::new(async move {
                PendingOnce::default().await;
                Ok(())
            })
        },
    );

    let module = Module::new(
        store.engine(),
        "
            (module
                (import \"\" \"\" (func $sync_call_async_thunk))
                (import \"\" \"\" (func $async_import))
                (func (export \"\")
                    ;; Set some store-local state and pointers
                    call $sync_call_async_thunk
                    ;; .. and hopefully it's all still configured correctly
                    call $async_import))
        ",
    )
    .unwrap();
    run(async {
        let instance = Instance::new_async(
            &store,
            &module,
            &[sync_call_async_thunk.into(), async_import.into()],
        )
        .await?;
        let func = instance.get_func("").unwrap();
        func.call_async(&[]).await
    })
    .unwrap();
}

#[test]
fn cancel_during_run() {
    let store = async_store();
    let state = Rc::new(Cell::new(0));
    let state2 = state.clone();

    let async_thunk = Func::new_async(
        &store,
        FuncType::new(None, None),
        (),
        move |_caller, _state, _params, _results| {
            assert_eq!(state2.get(), 0);
            state2.set(1);
            let dtor = SetOnDrop(state2.clone());
            Box::new(async move {
                drop(&dtor);
                PendingOnce::default().await;
                Ok(())
            })
        },
    );
    // Shouldn't have called anything yet...
    assert_eq!(state.get(), 0);

    // Create our future, but as per async conventions this still doesn't
    // actually do anything. No wasm or host function has been called yet.
    let mut future = Pin::from(Box::new(async_thunk.call_async(&[])));
    assert_eq!(state.get(), 0);

    // Push the future forward one tick, which actually runs the host code in
    // our async func. Our future is designed to be pending once, however.
    let poll = future
        .as_mut()
        .poll(&mut Context::from_waker(&dummy_waker()));
    assert!(poll.is_pending());
    assert_eq!(state.get(), 1);

    // Now that our future is running (on a separate, now-suspended fiber), drop
    // the future and that should deallocate all the Rust bits as well.
    drop(future);
    assert_eq!(state.get(), 2);

    struct SetOnDrop(Rc<Cell<u32>>);

    impl Drop for SetOnDrop {
        fn drop(&mut self) {
            assert_eq!(self.0.get(), 1);
            self.0.set(2);
        }
    }
}

#[derive(Default)]
struct PendingOnce {
    already_polled: bool,
}

impl Future for PendingOnce {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        if self.already_polled {
            Poll::Ready(())
        } else {
            self.already_polled = true;
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    }
}

fn run<F: Future>(future: F) -> F::Output {
    let mut f = Pin::from(Box::new(future));
    let waker = dummy_waker();
    let mut cx = Context::from_waker(&waker);
    loop {
        match f.as_mut().poll(&mut cx) {
            Poll::Ready(val) => break val,
            Poll::Pending => {}
        }
    }
}

fn dummy_waker() -> Waker {
    return unsafe { Waker::from_raw(clone(5 as *const _)) };

    unsafe fn clone(ptr: *const ()) -> RawWaker {
        assert_eq!(ptr as usize, 5);
        const VTABLE: RawWakerVTable = RawWakerVTable::new(clone, wake, wake_by_ref, drop);
        RawWaker::new(ptr, &VTABLE)
    }

    unsafe fn wake(ptr: *const ()) {
        assert_eq!(ptr as usize, 5);
    }

    unsafe fn wake_by_ref(ptr: *const ()) {
        assert_eq!(ptr as usize, 5);
    }

    unsafe fn drop(ptr: *const ()) {
        assert_eq!(ptr as usize, 5);
    }
}

#[test]
fn iloop_with_fuel() {
    let engine = Engine::new(Config::new().consume_fuel(true));
    let store = Store::new_async(&engine);
    store.out_of_fuel_async_yield(1_000, 10);
    let module = Module::new(
        &engine,
        "
            (module
                (func (loop br 0))
                (start 0)
            )
        ",
    )
    .unwrap();
    let instance = Instance::new_async(&store, &module, &[]);
    let mut f = Pin::from(Box::new(instance));
    let waker = dummy_waker();
    let mut cx = Context::from_waker(&waker);

    // This should yield a bunch of times...
    for _ in 0..100 {
        assert!(f.as_mut().poll(&mut cx).is_pending());
    }

    // ... but it should eventually also finish.
    loop {
        match f.as_mut().poll(&mut cx) {
            Poll::Ready(_) => break,
            Poll::Pending => {}
        }
    }
}

#[test]
fn fuel_eventually_finishes() {
    let engine = Engine::new(Config::new().consume_fuel(true));
    let store = Store::new_async(&engine);
    store.out_of_fuel_async_yield(u32::max_value(), 10);
    let module = Module::new(
        &engine,
        "
            (module
                (func
                    (local i32)
                    i32.const 100
                    local.set 0
                    (loop
                        local.get 0
                        i32.const -1
                        i32.add
                        local.tee 0
                        br_if 0)
                )
                (start 0)
            )
        ",
    )
    .unwrap();
    let instance = Instance::new_async(&store, &module, &[]);
    run(instance).unwrap();
}

#[test]
fn async_with_pooling_stacks() {
    let mut config = Config::new();
    config
        .with_allocation_strategy(InstanceAllocationStrategy::Pooling {
            strategy: PoolingAllocationStrategy::NextAvailable,
            module_limits: ModuleLimits {
                memory_pages: 1,
                table_elements: 0,
                ..Default::default()
            },
            instance_limits: InstanceLimits {
                count: 1,
                address_space_size: 1,
            },
        })
        .expect("pooling allocator created");

    let engine = Engine::new(&config);
    let store = Store::new_async(&engine);
    let func = Func::new_async(
        &store,
        FuncType::new(None, None),
        (),
        move |_caller, _state, _params, _results| Box::new(async { Ok(()) }),
    );
    run(func.call_async(&[])).unwrap();
    run(func.call_async(&[])).unwrap();
    let future1 = func.call_async(&[]);
    let future2 = func.call_async(&[]);
    run(future2).unwrap();
    run(future1).unwrap();
}
