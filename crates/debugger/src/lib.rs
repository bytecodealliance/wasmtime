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
/// states: running or paused. When paused, it acts as a
/// `StoreContextMut` and can allow examining the paused execution's
/// state. One runs until the next event suspends execution by
/// invoking `Debugger::run`.
///
/// Note that because of limitations in Wasmtime's future cancelation
/// handling, all events must be consumed until the inner body
/// completes and `Debugger::is_complete` returns
/// true. `Debugger::finish` continues execution ignoring all further
/// events to allow clean completion if needed.
pub struct Debugger<T: Send + 'static> {
    /// The inner task that this debugger wraps.
    inner: Option<JoinHandle<Result<Store<T>>>>,
    /// State: either a task handle or the store when passed out of
    /// the complete task.
    state: DebuggerState,
    in_tx: mpsc::Sender<Command<T>>,
    out_rx: mpsc::Receiver<Response>,
}

/// State machine from the perspective of the outer logic.
///
/// The intermediate states here, and the separation of these states
/// from the `JoinHandle` above, are what allow us to implement a
/// cancel-safe version of `Debugger::run` below.
///
/// The state diagram for the outer logic is:
///
/// ```plain
///              (start)
///                 v
///                 |
/// .--->---------. v
/// |     .----<  Paused  <-----------------------------------------------.
/// |     |         v                                                     |
/// |     |         | (async fn run() starts, sends Command::Continue)    |
/// |     |         |                                                     |
/// |     |         v                                                     ^
/// |     |      Running                                                  |
/// |     |       v v (async fn run() receives Response::Paused, returns) |
/// |     |       | |_____________________________________________________|
/// |     |       |
/// |     |       | (async fn run() receives Response::Finished, returns)
/// |     |       v
/// |     |     Complete
/// |     |
/// ^     | (async fn with_store() starts, sends Command::Query)
/// |     v
/// |   Queried
/// |     |
/// |     | (async fn with_store() receives Response::QueryResponse, returns)
/// `---<-'
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DebuggerState {
    /// Inner body is running in an async task and not in a debugger
    /// callback. Outer logic is waiting for a `Response::Paused` or
    /// `Response::Complete`.
    Running,
    /// Inner body is running in an async task and at a debugger
    /// callback (or in the initial trampoline waiting for the first
    /// `Continue`). `Response::Paused` has been received. Outer
    /// logic has not sent any commands.
    Paused,
    /// We have sent a command to the inner body and are waiting for a
    /// response.
    Queried,
    /// Inner body is complete (has sent `Response::Finished` and we
    /// have received it). We may or may not have joined yet; if so,
    /// the `Option<JoinHandle<...>>` will be `None`.
    Complete,
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
    Paused(DebugRunResult),
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
    async fn handle(&self, mut store: StoreContextMut<'_, T>, event: DebugEvent<'_>) {
        let mut in_rx = self.0.in_rx.lock().await;

        let result = match event {
            DebugEvent::HostcallError(_) => DebugRunResult::HostcallError,
            DebugEvent::CaughtExceptionThrown(exn) => DebugRunResult::CaughtExceptionThrown(exn),
            DebugEvent::UncaughtExceptionThrown(exn) => {
                DebugRunResult::UncaughtExceptionThrown(exn)
            }
            DebugEvent::Trap(trap) => DebugRunResult::Trap(trap),
            DebugEvent::Breakpoint => DebugRunResult::Breakpoint,
            DebugEvent::EpochYield => DebugRunResult::EpochYield,
        };
        self.0
            .out_tx
            .send(Response::Paused(result))
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

impl<T: Send + 'static> Debugger<T> {
    /// Create a new Debugger that attaches to the given Store and
    /// runs the given inner body.
    ///
    /// The debugger is always in one of two states: running or
    /// paused.
    ///
    /// When paused, the holder of this object can invoke
    /// `Debugger::run` to enter the running state. The inner body
    /// will run until paused by a debug event. While running, the
    /// future returned by either of these methods owns the `Debugger`
    /// and hence no other methods can be invoked.
    ///
    /// When paused, the holder of this object can access the `Store`
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
            inner: Some(inner),
            state: DebuggerState::Paused,
            in_tx,
            out_rx,
        }
    }

    /// Is the inner body done running?
    pub fn is_complete(&self) -> bool {
        match self.state {
            DebuggerState::Complete => true,
            _ => false,
        }
    }

    /// Run the inner body until the next debug event.
    ///
    /// This method is cancel-safe, and no events will be lost.
    pub async fn run(&mut self) -> Result<DebugRunResult> {
        log::trace!("running: state is {:?}", self.state);
        match self.state {
            DebuggerState::Paused => {
                log::trace!("sending Continue");
                self.in_tx
                    .send(Command::Continue)
                    .await
                    .map_err(|_| anyhow::anyhow!("Failed to send over debug channel"))?;
                log::trace!("sent Continue");

                // If that `send` was canceled, the command was not
                // sent, so it's fine to remain in `Paused`. If it
                // succeeded and we reached here, transition to
                // `Running` so we don't re-send.
                self.state = DebuggerState::Running;
            }
            DebuggerState::Running => {
                // Previous `run()` must have been canceled; no action
                // to take here.
            }
            DebuggerState::Queried => {
                // We expect to receive a `QueryResponse`; drop it if
                // the query was canceled, then transition back to
                // `Paused`.
                log::trace!("in Queried; receiving");
                let response = self
                    .out_rx
                    .recv()
                    .await
                    .ok_or_else(|| anyhow::anyhow!("Premature close of debugger channel"))?;
                log::trace!("in Queried; received, dropping");
                assert!(matches!(response, Response::QueryResponse(_)));
                self.state = DebuggerState::Paused;

                // Now send a `Continue`, as above.
                log::trace!("in Paused; sending Continue");
                self.in_tx
                    .send(Command::Continue)
                    .await
                    .map_err(|_| anyhow::anyhow!("Failed to send over debug channel"))?;
                self.state = DebuggerState::Running;
            }
            DebuggerState::Complete => {
                panic!("Cannot `run()` an already-complete Debugger");
            }
        }

        // At this point, the inner task is in Running state. We
        // expect to receive a message when it next pauses or
        // completes. If this `recv()` is canceled, no message is
        // lost, and the state above accurately reflects what must be
        // done on the next `run()`.
        log::trace!("waiting for response");
        let response = self
            .out_rx
            .recv()
            .await
            .ok_or_else(|| anyhow::anyhow!("Premature close of debugger channel"))?;

        match response {
            Response::Finished => {
                log::trace!("got Finished");
                self.state = DebuggerState::Complete;
                Ok(DebugRunResult::Finished)
            }
            Response::Paused(result) => {
                log::trace!("got Paused");
                self.state = DebuggerState::Paused;
                Ok(result)
            }
            Response::QueryResponse(_) => {
                panic!("Invalid debug response");
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
    ///
    /// This may only be invoked before the inner body finishes and
    /// when it is paused; that is, when the `Debugger` is initially
    /// created and after any call to `run()` returns a result other
    /// than `DebugRunResult::Finished`. If an earlier `run()`
    /// invocation was canceled, it must be re-invoked and return
    /// successfully before a query is made.
    ///
    /// This is cancel-safe; if canceled, the result of the query will
    /// be dropped.
    pub async fn with_store<
        F: FnOnce(StoreContextMut<'_, T>) -> R + Send + 'static,
        R: Send + 'static,
    >(
        &mut self,
        f: F,
    ) -> Result<R> {
        assert!(!self.is_complete());

        match self.state {
            DebuggerState::Queried => {
                // Earlier query canceled; drop its response first.
                let response = self
                    .out_rx
                    .recv()
                    .await
                    .ok_or_else(|| anyhow::anyhow!("Premature close of debugger channel"))?;
                assert!(matches!(response, Response::QueryResponse(_)));
                self.state = DebuggerState::Paused;
            }
            DebuggerState::Running => {
                // Results from a canceled `run()`; `run()` must
                // complete before this can be invoked.
                panic!("Cannot query in Running state");
            }
            DebuggerState::Complete => {
                panic!("Cannot query when complete");
            }
            DebuggerState::Paused => {
                // OK -- this is the state we want.
            }
        }

        self.in_tx
            .send(Command::Query(Box::new(|store| Box::new(f(store)))))
            .await
            .map_err(|_| anyhow::anyhow!("Premature close of debugger channel"))?;
        self.state = DebuggerState::Queried;

        let response = self
            .out_rx
            .recv()
            .await
            .ok_or_else(|| anyhow::anyhow!("Premature close of debugger channel"))?;
        let Response::QueryResponse(resp) = response else {
            anyhow::bail!("Incorrect response from debugger task");
        };
        self.state = DebuggerState::Paused;

        Ok(*resp.downcast::<R>().expect("type mismatch"))
    }

    /// Drop the Debugger once complete, returning the inner `Store`
    /// around which it was wrapped.
    ///
    /// Only valid to invoke once `run()` returns
    /// `DebugRunResult::Finished`.
    ///
    /// This is cancel-safe, but if canceled, the Store is lost.
    pub async fn take_store(&mut self) -> Result<Option<Store<T>>> {
        match self.state {
            DebuggerState::Complete => {
                let inner = match self.inner.take() {
                    Some(inner) => inner,
                    None => return Ok(None),
                };
                let mut store = inner.await??;
                store.clear_debug_handler();
                Ok(Some(store))
            }
            _ => panic!("Invalid state: debugger not yet complete"),
        }
    }
}

impl<T: Send + 'static> Drop for Debugger<T> {
    fn drop(&mut self) {
        // We cannot allow this because the fiber implementation will
        // panic if a `Func::call_async` future is dropped prematurely
        // -- in general, Wasmtime's futures that embody Wasm
        // execution are not cancel-safe, so we have to wait for the
        // inner body to finish before the Debugger is dropped.
        if self.state != DebuggerState::Complete {
            panic!("Dropping Debugger before inner body is complete");
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
    /// Wasm execution was interrupted by an epoch change.
    EpochYield,
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
        let mut store = debugger.take_store().await?.unwrap();
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
