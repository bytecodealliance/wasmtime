//! Wasmtime debugger functionality.
//!
//! This crate builds on top of the core Wasmtime crate's
//! guest-debugger APIs to present an environment where a debugger
//! runs as a "co-running process" and sees the debugee as a a
//! provider of a stream of events, on which actions can be taken
//! between each event.
//!
//! In the future, this crate will also provide a WIT-level API and
//! world in which to run debugger components.

use std::{any::Any, sync::Arc};
use tokio::{
    sync::{Mutex, mpsc},
    task::JoinHandle,
};
use wasmtime::{
    AsContextMut, DebugEvent, DebugHandler, ExnRef, OwnedRooted, Result, Store, StoreContextMut,
    Trap,
};

/// A `Debugger` wraps up state associated with debugging the code
/// running in a single `Store`.
///
/// It acts as a Future combinator, wrapping an inner async body that
/// performs some actions on a store. Those actions are subject to the
/// debugger, and debugger events will be raised as appropriate. From
/// the "outside" of this combinator, it is always in one of two
/// states: running or stopped. When stopped, it acts as a
/// `StoreContextMut` and can allow examining the stopped execution's
/// state. One runs until the next event suspends execution by
/// invoking `Debugger::run`.
///
/// Note that because of limitations in Wasmtime's future cancelation
/// handling, all events must be consumed until the inner body
/// completes and `Debugger::is_complete` returns
/// true. `Debugger::finish` continues execution ignoring all further
/// events to allow clean completion if needed.
pub struct Debugger<T: Send + 'static> {
    /// State: either a task handle or the store when passed out of
    /// the complete task.
    state: DebuggerState<T>,
    in_tx: mpsc::Sender<Command<T>>,
    out_rx: mpsc::Receiver<Response>,
}

enum DebuggerState<T: Send + 'static> {
    /// Inner body is running in an async task.
    Running(JoinHandle<Result<Store<T>>>),
    /// Temporary state while we are joining.
    Joining,
    /// Inner body is complete and has passed the store back.
    Complete(Store<T>),
    /// Debugger has been disassembled via `into_store()`. Allows the
    /// `Drop` impl to verify that the debugger is complete.
    Destructed,
}

/// Message from "outside" to the debug hook.
///
/// The `Query` catch-all with a boxed closure is a little janky, but
/// is the way that we provide access
/// from outside to the Store (which is owned by `inner` above)
/// only during pauses. Note that the future cannot take full
/// ownership or a mutable borrow of the Store, because it cannot
/// hold this across async yield points.
///
/// Instead, the debugger body sends boxed closures which take the
/// Store as a parameter (lifetime-limited not to escape that
/// closure) out to this crate's implementation that runs inside of
/// debugger-instrumentation callbacks (which have access to the
/// Store during their duration). We send return values
/// back. Return values are boxed Any values.
///
/// If we wanted to make this a little more principled, we could
/// come up with a Command/Response pair of enums for all possible
/// closures and make everything more statically typed and less
/// Box'd, but that would severely restrict the flexibility of the
/// abstraction here and essentially require writing a full proxy
/// of the debugger API.
///
/// Furthermore, we expect to rip this out eventually when we move
/// the debugger over to an async implementation based on
/// `run_concurrent` and `Accessor`s (see #11896). Building things
/// this way now will actually allow a less painful transition at
/// that time, because we will have a bunch of closures accessing
/// the store already and we can run those "with an accessor"
/// instead.
enum Command<T: 'static> {
    Continue,
    Query(Box<dyn FnOnce(StoreContextMut<'_, T>) -> Box<dyn Any + Send> + Send>),
}

enum Response {
    Stopped(DebugRunResult),
    QueryResponse(Box<dyn Any + Send>),
    Finished,
}

struct HandlerInner<T: Send + 'static> {
    in_rx: Mutex<mpsc::Receiver<Command<T>>>,
    out_tx: mpsc::Sender<Response>,
}

struct Handler<T: Send + 'static>(Arc<HandlerInner<T>>);

impl<T: Send + 'static> std::clone::Clone for Handler<T> {
    fn clone(&self) -> Self {
        Handler(self.0.clone())
    }
}

impl<T: Send + 'static> DebugHandler for Handler<T> {
    type Data = T;
    fn handle(
        &self,
        mut store: StoreContextMut<'_, T>,
        event: DebugEvent<'_>,
    ) -> impl Future<Output = ()> + Send {
        async move {
            let mut in_rx = self.0.in_rx.lock().await;

            let result = match event {
                DebugEvent::HostcallError(_) => DebugRunResult::HostcallError,
                DebugEvent::CaughtExceptionThrown(exn) => {
                    DebugRunResult::CaughtExceptionThrown(exn)
                }
                DebugEvent::UncaughtExceptionThrown(exn) => {
                    DebugRunResult::UncaughtExceptionThrown(exn)
                }
                DebugEvent::Trap(trap) => DebugRunResult::Trap(trap),
                DebugEvent::Breakpoint => DebugRunResult::Breakpoint,
            };
            self.0
                .out_tx
                .send(Response::Stopped(result))
                .await
                .expect("outbound channel closed prematurely");

            while let Some(cmd) = in_rx.recv().await {
                match cmd {
                    Command::Query(closure) => {
                        let result = closure(store.as_context_mut());
                        self.0
                            .out_tx
                            .send(Response::QueryResponse(result))
                            .await
                            .expect("outbound channel closed prematurely");
                    }
                    Command::Continue => {
                        break;
                    }
                }
            }
        }
    }
}

impl<T: Send + 'static> Debugger<T> {
    /// Create a new Debugger that attaches to the given Store and
    /// runs the given inner body.
    ///
    /// The debugger is always in one of two states: running or
    /// stopped.
    ///
    /// When stopped, the holder of this object can invoke
    /// `Debugger::run` to enter the running state. The inner body
    /// will run until stopped by a debug event. While running, the
    /// future returned by either of these methods owns the `Debugger`
    /// and hence no other methods can be invoked.
    ///
    /// When stopped, the holder of this object can access the `Store`
    /// indirectly by providing a closure
    pub fn new<F, I>(mut store: Store<T>, inner: F) -> Debugger<T>
    where
        I: Future<Output = Result<Store<T>>> + Send + 'static,
        F: for<'a> FnOnce(Store<T>) -> I + Send + 'static,
    {
        let (in_tx, mut in_rx) = mpsc::channel(1);
        let (out_tx, out_rx) = mpsc::channel(1);

        let inner = tokio::spawn(async move {
            // Receive one "continue" command on the inbound channel
            // before continuing.
            match in_rx.recv().await {
                Some(cmd) => {
                    assert!(matches!(cmd, Command::Continue));
                }
                None => {
                    // Premature exit due to closed channel. Just drop `inner`.
                    anyhow::bail!("Debugger channel dropped");
                }
            }

            let out_tx_clone = out_tx.clone();
            store.set_debug_handler(Handler(Arc::new(HandlerInner {
                in_rx: Mutex::new(in_rx),
                out_tx,
            })));
            let result = inner(store).await;
            let _ = out_tx_clone.send(Response::Finished).await;
            result
        });

        Debugger {
            state: DebuggerState::Running(inner),
            in_tx,
            out_rx,
        }
    }

    /// Is the inner body done running?
    pub fn is_complete(&self) -> bool {
        match &self.state {
            DebuggerState::Running(_) | DebuggerState::Joining => false,
            DebuggerState::Complete(_) => true,
            DebuggerState::Destructed => {
                panic!("Should not see this state outside of `into_store()`")
            }
        }
    }

    /// Run the inner body until the next debug event.
    pub async fn run(&mut self) -> Result<DebugRunResult> {
        anyhow::ensure!(!self.is_complete(), "Debugger body is already complete");

        self.in_tx
            .send(Command::Continue)
            .await
            .map_err(|_| anyhow::anyhow!("Failed to send over debug channel"))?;

        let response = self
            .out_rx
            .recv()
            .await
            .ok_or_else(|| anyhow::anyhow!("Premature close of debugger channel"))?;

        match response {
            Response::Finished => {
                let DebuggerState::Running(joinhandle) =
                    std::mem::replace(&mut self.state, DebuggerState::Joining)
                else {
                    panic!("State was verified to be `Running` above");
                };
                let store = joinhandle.await??;
                self.state = DebuggerState::Complete(store);
                Ok(DebugRunResult::Finished)
            }
            Response::Stopped(result) => Ok(result),
            Response::QueryResponse(_) => {
                anyhow::bail!("Invalid debug response");
            }
        }
    }

    /// Run the debugger body until completion, with no further events.
    pub async fn finish(&mut self) -> Result<()> {
        if self.is_complete() {
            return Ok(());
        }
        loop {
            match self.run().await? {
                DebugRunResult::Finished => break,
                e => {
                    log::trace!("finish: event {e:?}");
                }
            }
        }
        assert!(self.is_complete());
        Ok(())
    }

    /// Perform some action on the contained `Store` while not running.
    pub async fn with_store<
        F: FnOnce(StoreContextMut<'_, T>) -> R + Send + 'static,
        R: Send + 'static,
    >(
        &mut self,
        f: F,
    ) -> Result<R> {
        match &mut self.state {
            DebuggerState::Running(_) => {
                self.in_tx
                    .send(Command::Query(Box::new(|store| Box::new(f(store)))))
                    .await
                    .map_err(|_| anyhow::anyhow!("Premature close of debugger channel"))?;
                let response = self
                    .out_rx
                    .recv()
                    .await
                    .ok_or_else(|| anyhow::anyhow!("Premature close of debugger channel"))?;
                let Response::QueryResponse(resp) = response else {
                    anyhow::bail!("Incorrect response from debugger task");
                };
                Ok(*resp.downcast::<R>().expect("type mismatch"))
            }
            DebuggerState::Joining => anyhow::bail!("Join failed with error and Store is lost"),
            DebuggerState::Complete(store) => Ok(f(store.as_context_mut())),
            DebuggerState::Destructed => {
                panic!("Should not see `Destructed` state outside of `into_store`")
            }
        }
    }

    /// Drop the Debugger once complete, returning the inner `Store`
    /// around which it was wrapped.
    pub fn into_store(mut self) -> Store<T> {
        let state = std::mem::replace(&mut self.state, DebuggerState::Destructed);
        let mut store = match state {
            DebuggerState::Complete(store) => store,
            _ => panic!("Cannot invoke into_store() on a non-complete Debugger"),
        };
        store.clear_debug_handler();
        store
    }
}

impl<T: Send + 'static> Drop for Debugger<T> {
    fn drop(&mut self) {
        // We cannot allow this because the fiber implementation will
        // panic if a `Func::call_async` future is dropped prematurely
        // -- in general, Wasmtime's futures that embody Wasm
        // execution are not cancel-safe, so we have to wait for the
        // inner body to finish before the Debugger is dropped.
        match &self.state {
            DebuggerState::Complete(_) | DebuggerState::Destructed => {}
            _ => panic!("Dropping Debugger before inner body is complete"),
        }
    }
}

/// The result of one call to `Debugger::run()`.
///
/// This is similar to `DebugEvent` but without the lifetime, so it
/// can be sent across async tasks, and incorporates the possibility
/// of completion (`Finished`) as well.
#[derive(Debug)]
pub enum DebugRunResult {
    /// Execution of the inner body finished.
    Finished,
    /// An error was raised by a hostcall.
    HostcallError,
    /// An exception is thrown and caught by Wasm. The current state
    /// is at the throw-point.
    CaughtExceptionThrown(OwnedRooted<ExnRef>),
    /// An exception was not caught and is escaping to the host.
    UncaughtExceptionThrown(OwnedRooted<ExnRef>),
    /// A Wasm trap occurred.
    Trap(Trap),
    /// A breakpoint was reached.
    Breakpoint,
}

#[cfg(test)]
mod test {
    use super::*;
    use wasmtime::*;

    #[tokio::test]
    #[cfg_attr(miri, ignore)]
    async fn basic_debugger() -> anyhow::Result<()> {
        let _ = env_logger::try_init();

        let mut config = Config::new();
        config.guest_debug(true);
        config.async_support(true);
        let engine = Engine::new(&config)?;
        let module = Module::new(
            &engine,
            r#"
                (module
                  (func (export "main") (param i32 i32) (result i32)
                    local.get 0
                    local.get 1
                    i32.add))
            "#,
        )?;

        let mut store = Store::new(&engine, ());
        let instance = Instance::new_async(&mut store, &module, &[]).await?;
        let main = instance.get_func(&mut store, "main").unwrap();

        let mut debugger = Debugger::new(store, move |mut store| async move {
            let mut results = [Val::I32(0)];
            store.edit_breakpoints().unwrap().single_step(true).unwrap();
            main.call_async(&mut store, &[Val::I32(1), Val::I32(2)], &mut results[..])
                .await?;
            assert_eq!(results[0].unwrap_i32(), 3);
            main.call_async(&mut store, &[Val::I32(3), Val::I32(4)], &mut results[..])
                .await?;
            assert_eq!(results[0].unwrap_i32(), 7);
            Ok(store)
        });

        let event = debugger.run().await?;
        assert!(matches!(event, DebugRunResult::Breakpoint));
        // At (before executing) first `local.get`.
        debugger
            .with_store(|store| {
                let mut frame = store.debug_frames().unwrap();
                assert!(!frame.done());
                assert_eq!(frame.wasm_function_index_and_pc().unwrap().0.as_u32(), 0);
                assert_eq!(frame.wasm_function_index_and_pc().unwrap().1, 36);
                assert_eq!(frame.num_locals(), 2);
                assert_eq!(frame.num_stacks(), 0);
                assert_eq!(frame.local(0).unwrap_i32(), 1);
                assert_eq!(frame.local(1).unwrap_i32(), 2);
                assert_eq!(frame.move_to_parent(), FrameParentResult::SameActivation);
                assert!(frame.done());
            })
            .await?;

        let event = debugger.run().await?;
        // At second `local.get`.
        assert!(matches!(event, DebugRunResult::Breakpoint));
        debugger
            .with_store(|store| {
                let mut frame = store.debug_frames().unwrap();
                assert!(!frame.done());
                assert_eq!(frame.wasm_function_index_and_pc().unwrap().0.as_u32(), 0);
                assert_eq!(frame.wasm_function_index_and_pc().unwrap().1, 38);
                assert_eq!(frame.num_locals(), 2);
                assert_eq!(frame.num_stacks(), 1);
                assert_eq!(frame.local(0).unwrap_i32(), 1);
                assert_eq!(frame.local(1).unwrap_i32(), 2);
                assert_eq!(frame.stack(0).unwrap_i32(), 1);
                assert_eq!(frame.move_to_parent(), FrameParentResult::SameActivation);
                assert!(frame.done());
            })
            .await?;

        let event = debugger.run().await?;
        // At `i32.add`.
        assert!(matches!(event, DebugRunResult::Breakpoint));
        debugger
            .with_store(|store| {
                let mut frame = store.debug_frames().unwrap();
                assert!(!frame.done());
                assert_eq!(frame.wasm_function_index_and_pc().unwrap().0.as_u32(), 0);
                assert_eq!(frame.wasm_function_index_and_pc().unwrap().1, 40);
                assert_eq!(frame.num_locals(), 2);
                assert_eq!(frame.num_stacks(), 2);
                assert_eq!(frame.local(0).unwrap_i32(), 1);
                assert_eq!(frame.local(1).unwrap_i32(), 2);
                assert_eq!(frame.stack(0).unwrap_i32(), 1);
                assert_eq!(frame.stack(1).unwrap_i32(), 2);
                assert_eq!(frame.move_to_parent(), FrameParentResult::SameActivation);
                assert!(frame.done());
            })
            .await?;

        let event = debugger.run().await?;
        // At return point.
        assert!(matches!(event, DebugRunResult::Breakpoint));
        debugger
            .with_store(|store| {
                let mut frame = store.debug_frames().unwrap();
                assert!(!frame.done());
                assert_eq!(frame.wasm_function_index_and_pc().unwrap().0.as_u32(), 0);
                assert_eq!(frame.wasm_function_index_and_pc().unwrap().1, 41);
                assert_eq!(frame.num_locals(), 2);
                assert_eq!(frame.num_stacks(), 1);
                assert_eq!(frame.local(0).unwrap_i32(), 1);
                assert_eq!(frame.local(1).unwrap_i32(), 2);
                assert_eq!(frame.stack(0).unwrap_i32(), 3);
                assert_eq!(frame.move_to_parent(), FrameParentResult::SameActivation);
                assert!(frame.done());
            })
            .await?;

        // Now disable breakpoints before continuing. Second call should proceed with no more events.
        debugger
            .with_store(|store| {
                store
                    .edit_breakpoints()
                    .unwrap()
                    .single_step(false)
                    .unwrap();
            })
            .await?;

        let event = debugger.run().await?;
        assert!(matches!(event, DebugRunResult::Finished));

        assert!(debugger.is_complete());

        // Ensure the store still works and the debug handler is
        // removed.
        let mut store = debugger.into_store();
        let mut results = [Val::I32(0)];
        main.call_async(&mut store, &[Val::I32(10), Val::I32(20)], &mut results[..])
            .await?;
        assert_eq!(results[0].unwrap_i32(), 30);

        Ok(())
    }

    #[tokio::test]
    #[cfg_attr(miri, ignore)]
    async fn early_finish() -> Result<()> {
        let _ = env_logger::try_init();

        let mut config = Config::new();
        config.guest_debug(true);
        config.async_support(true);
        let engine = Engine::new(&config)?;
        let module = Module::new(
            &engine,
            r#"
                (module
                  (func (export "main") (param i32 i32) (result i32)
                    local.get 0
                    local.get 1
                    i32.add))
            "#,
        )?;

        let mut store = Store::new(&engine, ());
        let instance = Instance::new_async(&mut store, &module, &[]).await?;
        let main = instance.get_func(&mut store, "main").unwrap();

        let mut debugger = Debugger::new(store, move |mut store| async move {
            let mut results = [Val::I32(0)];
            store.edit_breakpoints().unwrap().single_step(true).unwrap();
            main.call_async(&mut store, &[Val::I32(1), Val::I32(2)], &mut results[..])
                .await?;
            assert_eq!(results[0].unwrap_i32(), 3);
            Ok(store)
        });

        debugger.finish().await?;
        assert!(debugger.is_complete());

        Ok(())
    }
}
