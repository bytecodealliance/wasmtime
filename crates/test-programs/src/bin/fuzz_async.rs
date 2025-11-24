//! Test case used with the `component_async` fuzzer which is part of the `misc`
//! fuzz target of Wasmtime.
//!
//! This test case is a binary that's suited for just that one fuzzer and has an
//! associated WIT world that it works with. This test case is composed with
//! itself and then run within the host as well. The exact semantics of this
//! program and all the exports/imports are defined within the context of the
//! fuzzer.
//!
//! The general idea is that this program creates an "async soup" and make sure
//! that everything works as expected, notably also not leading to any panics
//! anywhere within the runtime. An example of what this fuzzer intermingles
//! are:
//!
//! * Synchronous calls
//! * Async calls that are immediately ready
//! * Async calls that are not immediately ready and left pending
//! * Creation of futures/streams
//! * Moving futures/streams between components
//! * Reading/writing futures/streams
//! * Cancelling reads/writes of futures/streams
//! * Seeing futures/streams get dropped and the effect on active reads/writes
//! * Mixing host<->guest, guest<->guest, guest<->host, and host<->host
//!   calls/primitives
//!
//! The purpose of this fuzzer is not stress the management of async stacks, the
//! async runtime, and in theory suss out various edge cases in the handling of
//! async events. This fuzzer does NOT stress lifting/lowering at all because
//! there is a static WIT signature that this fuzzer works with.
//!
//! Much of the code in this file is semi-duplicated in the host except written
//! with host `wasmtime` APIs instead of `wit-bindgen` APIs. The overall
//! structure is roughly the same.
//!
//! # Overall architecture
//!
//! The general structure of this fuzzer is that there's a "host sandwich" which
//! looks like:
//!
//! ```text
//! ╔══════╦══════════════════════════════════════════════════════════╗
//! ║ Host ║                                                          ║
//! ╠══════╝                                                          ║
//! ║                                                                 ║
//! ║ ┍┯┯┯━━━ wasmtime:fuzz/types                                     ║
//! ║ ││││                                                            ║
//! ║ ││││                                                            ║
//! ║ ││││        ╔════════════════════╦════════════════════╗         ║
//! ║ ││││        ║ component_async.rs ║                    ║         ║
//! ║ ││││        ╠════════════════════╝                    ║         ║
//! ║ ││││        ║                                         ║         ║
//! ║ ││││        ║            HostCaller                   ║         ║
//! ║ ││││        ║                                         ║         ║
//! ║ │││└────────╫─→ stream<command>                       ║         ║
//! ║ │││         ╚═══════════════════╤═════════════════════╝         ║
//! ║ │││                             │                               ║
//! ║ │││                             ┝ wasmtime-fuzz:fuzz/async-test ║
//! ║ │││                             │                               ║
//! ║ │││    ╔═══════════╦════════════╪═════════════════════════════╗ ║
//! ║ │││    ║ Component ║            │                             ║ ║
//! ║ │││    ╠═══════════╝            │                             ║ ║
//! ║ │││    ║                        ↓                             ║ ║
//! ║ │││    ║    ╔═════════════════╦═══════════════════════╗       ║ ║
//! ║ │││    ║    ║ fuzz-async.wasm ║                       ║       ║ ║
//! ║ │││    ║    ╠═════════════════╝                       ║       ║ ║
//! ║ │││    ║    ║                                         ║       ║ ║
//! ║ │││    ║    ║            GuestCaller                  ║       ║ ║
//! ║ │││    ║    ║                                         ║       ║ ║
//! ║ ││└────╫────╫─→ stream<command>                       ║       ║ ║
//! ║ ││     ║    ╚═══════╤═════════════════════════════════╝       ║ ║
//! ║ ││     ║            │                                         ║ ║
//! ║ ││     ║            ┝ wasmtime-fuzz:fuzz/async-test           ║ ║
//! ║ ││     ║            │                                         ║ ║
//! ║ ││     ║            ↓                                         ║ ║
//! ║ ││     ║    ╔═════════════════╦═══════════════════════╗       ║ ║
//! ║ ││     ║    ║ fuzz-async.wasm ║                       ║       ║ ║
//! ║ ││     ║    ╠═════════════════╝                       ║       ║ ║
//! ║ ││     ║    ║                                         ║       ║ ║
//! ║ ││     ║    ║            GuestCallee                  ║       ║ ║
//! ║ ││     ║    ║                                         ║       ║ ║
//! ║ │└─────╫────╫─→ stream<command>                       ║       ║ ║
//! ║ │      ║    ╚═══════════════════╤═════════════════════╝       ║ ║
//! ║ │      ║                        │                             ║ ║
//! ║ │      ║                        │                             ║ ║
//! ║ │      ╚════════════════════════╪═════════════════════════════╝ ║
//! ║ │                               │                               ║
//! ║ │                               ┝ wasmtime-fuzz:fuzz/async-test ║
//! ║ │                               │                               ║
//! ║ │                               ↓                               ║
//! ║ │           ╔════════════════════╦════════════════════╗         ║
//! ║ │           ║ component_async.rs ║                    ║         ║
//! ║ │           ╠════════════════════╝                    ║         ║
//! ║ │           ║                                         ║         ║
//! ║ │           ║            HostCallee                   ║         ║
//! ║ │           ║                                         ║         ║
//! ║ └───────────╫─→ stream<command>                       ║         ║
//! ║             ╚═════════════════════════════════════════╝         ║
//! ║                                                                 ║
//! ╚═════════════════════════════════════════════════════════════════╝
//! ```
//!
//! Here `fuzz-async.wasm` appears twice to model all the various types of
//! the host/guest interaction matrix. Everything is driven by a
//! `stream<command>` provided to each component part of the system which
//! serves as a means of forcing one particular component to take action.
//! Commands are then the test case itself where a series of commands are
//! executed for each fuzz iteration.
//!
//! # Yield-loops
//!
//! This program has a function `test_property` which is a similar analog to the
//! one in the host-side as well. The general idea is that while component model
//! async is generally deterministic it does not specify what should happen when
//! multiple events are ready at the same time. This can pretty easily happen in
//! this fuzzer meaning that it's not precise which event happens first. To
//! assist in managing this there are two primary mitigations:
//!
//! * The first is that whenever a command is dispatched to a component it's
//!   followed up with an "ack" which is a noop. Delivery of the "ack" can't
//!   happen until the previous command is completely finished being processed
//!   meaning it's a kludge way of synchronizing the receipt of a message.
//!
//! * The second is that there can still be small races where an async event
//!   hasn't quite happen yet but it's queued up to happen. To handle these
//!   events calls to `test_property` are sprinkled around which has an
//!   internally-bounded yield-loop. It's expected that while yielding other
//!   code can run which resolves the property being tested at-hand, and then
//!   this yield loop will panic if it turns too many times as it's probably a
//!   bug.
//!
//! It's a bit of a hack but it's so far the most effective way of handling this
//! that's (a) not timing-dependent e.g. adding sleeps, (b) is
//! reliable/deterministic, and (c) is flexible where the constant number of
//! yields can be bumped without much concern. The number of yields specifically
//! is arbitrarily chosen and while it can't be said exactly how many yields
//! should be necessary it should be able to say "less than N should always
//! work".

wit_bindgen::generate!("fuzz-async" in "../fuzzing/wit");

use crate::exports::wasmtime_fuzz::fuzz::async_test as e;
use crate::wasmtime_fuzz::fuzz::async_test as i;
use crate::wasmtime_fuzz::fuzz::types::{self, Command, Scope};
use futures::FutureExt;
use futures::channel::oneshot;
use pin_project_lite::pin_project;
use std::collections::{HashMap, HashSet};
use std::mem;
use std::pin::{Pin, pin};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::task::{Context, Poll, Waker};
use wit_bindgen::{FutureReader, FutureWriter, StreamReader, StreamResult, StreamWriter};

struct Component;

export!(Component);

// Convenience macro to change the "target" of `log::debug!` based on whether
// this component is a `caller` or `callee` scope to distinguish logs in the
// output.
macro_rules! debug {
    ($($arg:tt)*) => {
        log::debug!(target: log_target(), $($arg)*);
    }
}

static IS_CALLER: AtomicBool = AtomicBool::new(false);

fn log_target() -> &'static str {
    if IS_CALLER.load(Ordering::Relaxed) {
        "wasmtime_fuzzing::fuzz_async::caller"
    } else {
        "wasmtime_fuzzing::fuzz_async::callee"
    }
}

impl e::Guest for Component {
    fn sync_ready() {}

    async fn async_ready() {}

    async fn async_pending(id: u32) {
        let (tx, rx) = oneshot::channel();
        State::with(|s| s.async_pending_exports_ready.insert(id, tx));
        let record = RecordCancelOnDrop(id);
        rx.await.unwrap();
        mem::forget(record);
        debug!("export {id} is complete");

        struct RecordCancelOnDrop(u32);

        impl Drop for RecordCancelOnDrop {
            fn drop(&mut self) {
                debug!("export {} was cancelled", self.0);
                State::with(|s| {
                    s.async_pending_exports_cancelled.insert(self.0);
                });
            }
        }
    }

    async fn init(scope: Scope) {
        IS_CALLER.store(scope == Scope::Caller, Ordering::Relaxed);
        env_logger::init();
        i::init(Scope::Callee).await;
        let commands = types::get_commands(scope);
        wit_bindgen::spawn(run(commands));
    }

    fn future_take(id: u32) -> FutureReader<u32> {
        State::with(|s| s.future_readers.remove(&id).unwrap())
    }

    fn future_receive(id: u32, f: FutureReader<u32>) {
        let prev = State::with(|s| s.future_readers.insert(id, f));
        assert!(prev.is_none());
    }

    fn stream_take(id: u32) -> StreamReader<u32> {
        State::with(|s| s.stream_readers.remove(&id).unwrap())
    }

    fn stream_receive(id: u32, f: StreamReader<u32>) {
        let prev = State::with(|s| s.stream_readers.insert(id, f));
        assert!(prev.is_none());
    }
}

#[derive(Default)]
struct State {
    async_pending_imports_ready: HashSet<u32>,
    async_pending_imports_in_progress: HashMap<u32, oneshot::Sender<()>>,
    async_pending_exports_ready: HashMap<u32, oneshot::Sender<()>>,
    async_pending_exports_cancelled: HashSet<u32>,

    future_readers: HashMap<u32, FutureReader<u32>>,
    future_writers: HashMap<u32, FutureWriter<u32>>,
    future_write_cancel_signals: HashMap<u32, oneshot::Sender<()>>,
    future_read_cancel_signals: HashMap<u32, oneshot::Sender<()>>,
    future_writes_completed: HashMap<u32, bool>,
    future_reads_completed: HashMap<u32, u32>,

    stream_readers: HashMap<u32, StreamReader<u32>>,
    stream_writers: HashMap<u32, StreamWriter<u32>>,
    stream_write_cancel_signals: HashMap<u32, oneshot::Sender<()>>,
    stream_read_cancel_signals: HashMap<u32, oneshot::Sender<()>>,
    stream_writes_completed: HashMap<u32, Result<(usize, Vec<u32>), (usize, Vec<u32>)>>,
    stream_reads_completed: HashMap<u32, Option<Vec<u32>>>,
}

impl State {
    pub fn with<R>(f: impl FnOnce(&mut State) -> R) -> R {
        static STATE: Mutex<Option<State>> = Mutex::new(None);
        let mut state = STATE.lock().unwrap();
        let state = state.get_or_insert_with(|| State::default());
        f(state)
    }

    pub async fn test_property(mut f: impl FnMut(&mut State) -> bool) -> bool {
        // Test if the property is ready, but it might require a sibling future
        // task to run, so if it's not true yet then pump the executor a
        // few times to let it finish.
        for _ in 0..1000 {
            if State::with(&mut f) {
                return true;
            }
            wit_bindgen::yield_async().await;
        }
        return false;
    }
}

async fn run(mut commands: StreamReader<Command>) {
    while let Some(command) = commands.next().await {
        match command {
            Command::SyncReadyCall => i::sync_ready(),

            Command::AsyncReadyCall => assert_ready(pin!(i::async_ready())),

            Command::AsyncPendingExportComplete(i) => {
                assert!(
                    State::test_property(|s| s.async_pending_exports_ready.contains_key(&i)).await,
                    "expected async_pending export {i} should be pending",
                );
                debug!("finishing export {i}");
                State::with(|s| {
                    s.async_pending_exports_ready
                        .remove(&i)
                        .unwrap()
                        .send(())
                        .unwrap();
                });
            }
            Command::AsyncPendingExportAssertCancelled(i) => {
                assert!(
                    State::test_property(|s| s.async_pending_exports_cancelled.remove(&i)).await,
                    "expected async_pending export {i} to be cancelled",
                );
            }
            Command::AsyncPendingImportCall(i) => {
                let mut future = Box::pin(i::async_pending(i));
                debug!("starting export {i}");
                assert_not_ready(future.as_mut());
                let (cancel_tx, mut cancel_rx) = oneshot::channel();
                State::with(|s| {
                    s.async_pending_imports_in_progress.insert(i, cancel_tx);
                });
                wit_bindgen::spawn(async move {
                    futures::select! {
                        _ = cancel_rx => {}
                        _ = future.fuse() => {
                            State::with(|s| s.async_pending_imports_ready.insert(i));
                        }
                    }
                });
            }
            Command::AsyncPendingImportCancel(i) => {
                debug!("cancelling import {i}");
                State::with(|s| {
                    s.async_pending_imports_in_progress
                        .remove(&i)
                        .unwrap()
                        .send(())
                        .unwrap();
                });
            }
            Command::AsyncPendingImportAssertReady(i) => {
                assert!(
                    State::test_property(|s| s.async_pending_imports_ready.remove(&i)).await,
                    "expected async_pending import {i} to be ready",
                );
            }

            Command::FutureNew(id) => {
                let (writer, reader) = wit_future::new(|| unreachable!());
                State::with(|s| {
                    let prev = s.future_writers.insert(id, writer);
                    assert!(prev.is_none());
                    let prev = s.future_readers.insert(id, reader);
                    assert!(prev.is_none());
                });
            }
            Command::FutureTake(id) => {
                let reader = i::future_take(id);
                State::with(|s| {
                    let prev = s.future_readers.insert(id, reader);
                    assert!(prev.is_none());
                });
            }
            Command::FutureGive(id) => {
                let reader = State::with(|s| s.future_readers.remove(&id).unwrap());
                i::future_receive(id, reader);
            }
            Command::FutureDropReadable(id) => {
                let _ = State::with(|s| s.future_readers.remove(&id).unwrap());
            }
            Command::FutureWriteReady(payload) => {
                let writer = State::with(|s| s.future_writers.remove(&payload.future).unwrap());
                assert_ready(pin!(writer.write(payload.item))).unwrap();
            }
            Command::FutureReadReady(payload) => {
                let reader = State::with(|s| s.future_readers.remove(&payload.future).unwrap());
                assert_eq!(assert_ready(pin!(reader.into_future())), payload.item);
            }
            Command::FutureWriteDropped(id) => {
                let writer = State::with(|s| s.future_writers.remove(&id).unwrap());
                match assert_ready(pin!(writer.write(0))) {
                    Ok(_) => panic!("should be dropped"),
                    Err(_) => {}
                }
            }
            Command::FutureWritePending(payload) => {
                use wit_bindgen::FutureWriteCancel;

                let writer = State::with(|s| s.future_writers.remove(&payload.future).unwrap());
                let (tx, rx) = oneshot::channel();
                let mut future = Box::pin(CancellableFutureWrite {
                    cancel: rx,
                    write: writer.write(payload.item),
                });
                assert_not_ready(future.as_mut());
                wit_bindgen::spawn(async move {
                    let result = future.await;
                    debug!("future write {} completed: {result:?}", payload.future);
                    State::with(|s| match result {
                        FutureWriteCancel::AlreadySent => {
                            s.future_writes_completed.insert(payload.future, true);
                        }
                        FutureWriteCancel::Dropped(_) => {
                            s.future_writes_completed.insert(payload.future, false);
                        }
                        FutureWriteCancel::Cancelled(_, writer) => {
                            let prev = s.future_writers.insert(payload.future, writer);
                            assert!(prev.is_none());
                        }
                    });
                });
                State::with(|s| {
                    let prev = s.future_write_cancel_signals.insert(payload.future, tx);
                    assert!(prev.is_none());
                });

                pin_project! {
                    struct CancellableFutureWrite {
                        #[pin]
                        cancel: oneshot::Receiver<()>,
                        #[pin]
                        write: wit_bindgen::FutureWrite<u32>,
                    }
                }

                impl Future for CancellableFutureWrite {
                    type Output = FutureWriteCancel<u32>;

                    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
                        let this = self.project();
                        match this.cancel.poll(cx) {
                            Poll::Ready(_) => return Poll::Ready(this.write.cancel()),
                            Poll::Pending => {}
                        }
                        match this.write.poll(cx) {
                            Poll::Ready(Ok(())) => Poll::Ready(FutureWriteCancel::AlreadySent),
                            Poll::Ready(Err(val)) => {
                                Poll::Ready(FutureWriteCancel::Dropped(val.value))
                            }
                            Poll::Pending => Poll::Pending,
                        }
                    }
                }
            }
            Command::FutureReadPending(id) => {
                let reader = State::with(|s| s.future_readers.remove(&id).unwrap());
                let (tx, rx) = oneshot::channel();
                let mut future = Box::pin(CancellableFutureRead {
                    cancel: rx,
                    read: reader.into_future(),
                });
                assert_not_ready(future.as_mut());
                wit_bindgen::spawn(async move {
                    let result = future.await;
                    State::with(|s| match result {
                        Ok(result) => {
                            let prev = s.future_reads_completed.insert(id, result);
                            assert!(prev.is_none());
                        }
                        Err(reader) => {
                            let prev = s.future_readers.insert(id, reader);
                            assert!(prev.is_none());
                        }
                    });
                });
                State::with(|s| {
                    let prev = s.future_read_cancel_signals.insert(id, tx);
                    assert!(prev.is_none());
                });

                pin_project! {
                    struct CancellableFutureRead {
                        #[pin]
                        cancel: oneshot::Receiver<()>,
                        #[pin]
                        read: wit_bindgen::FutureRead<u32>,
                    }
                }

                impl Future for CancellableFutureRead {
                    type Output = Result<u32, FutureReader<u32>>;

                    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
                        let this = self.project();
                        match this.cancel.poll(cx) {
                            Poll::Ready(_) => return Poll::Ready(this.read.cancel()),
                            Poll::Pending => {}
                        }
                        match this.read.poll(cx) {
                            Poll::Ready(i) => Poll::Ready(Ok(i)),
                            Poll::Pending => Poll::Pending,
                        }
                    }
                }
            }
            Command::FutureCancelWrite(id) => {
                State::with(|s| {
                    s.future_write_cancel_signals
                        .remove(&id)
                        .unwrap()
                        .send(())
                        .unwrap();
                });
                assert!(
                    State::test_property(|s| s.future_writers.contains_key(&id)).await,
                    "expected future write {id} to be cancelled",
                );
            }
            Command::FutureCancelRead(id) => {
                State::with(|s| {
                    s.future_read_cancel_signals
                        .remove(&id)
                        .unwrap()
                        .send(())
                        .unwrap();
                });
                assert!(
                    State::test_property(|s| s.future_readers.contains_key(&id)).await,
                    "expected future read {id} to be cancelled",
                );
            }
            Command::FutureWriteAssertComplete(id) => {
                assert!(
                    State::test_property(|s| match s.future_writes_completed.remove(&id) {
                        Some(true) => true,
                        Some(false) => panic!("future was dropped"),
                        None => false,
                    })
                    .await,
                    "expected future write {id} to be complete",
                );
            }
            Command::FutureWriteAssertDropped(id) => {
                assert!(
                    State::test_property(|s| match s.future_writes_completed.remove(&id) {
                        Some(true) => panic!("future write completed"),
                        Some(false) => true,
                        None => false,
                    })
                    .await,
                    "expected future write {id} to be complete",
                );
            }
            Command::FutureReadAssertComplete(payload) => {
                assert!(
                    State::test_property(|s| {
                        match s.future_reads_completed.remove(&payload.future) {
                            Some(i) => {
                                assert_eq!(i, payload.item);
                                true
                            }
                            None => false,
                        }
                    })
                    .await,
                    "expected future read {} to be complete",
                    payload.future,
                );
            }

            Command::StreamNew(id) => {
                let (writer, reader) = wit_stream::new();
                State::with(|s| {
                    let prev = s.stream_writers.insert(id, writer);
                    assert!(prev.is_none());
                    let prev = s.stream_readers.insert(id, reader);
                    assert!(prev.is_none());
                });
            }
            Command::StreamTake(id) => {
                let reader = i::stream_take(id);
                State::with(|s| {
                    let prev = s.stream_readers.insert(id, reader);
                    assert!(prev.is_none());
                });
            }
            Command::StreamGive(id) => {
                let reader = State::with(|s| s.stream_readers.remove(&id).unwrap());
                i::stream_receive(id, reader);
            }
            Command::StreamDropReadable(id) => {
                let _ = State::with(|s| s.stream_readers.remove(&id).unwrap());
            }
            Command::StreamDropWritable(id) => {
                let _ = State::with(|s| s.stream_writers.remove(&id).unwrap());
            }
            Command::StreamWriteReady(payload) => {
                State::with(|s| {
                    let writer = s.stream_writers.get_mut(&payload.stream).unwrap();
                    let (status, buffer) = assert_ready(pin!(
                        writer.write(stream_payload(payload.item, payload.op_count))
                    ));
                    assert_eq!(status, StreamResult::Complete(payload.ready_count as usize));
                    assert_eq!(
                        buffer.remaining() as u32,
                        payload.op_count - payload.ready_count
                    );
                });
            }
            Command::StreamWriteDropped(payload) => {
                State::with(|s| {
                    let writer = s.stream_writers.get_mut(&payload.stream).unwrap();
                    let (status, buffer) = assert_ready(pin!(
                        writer.write(stream_payload(payload.item, payload.count))
                    ));
                    assert_eq!(status, StreamResult::Dropped);
                    assert_eq!(buffer.remaining() as u32, payload.count);
                });
            }
            Command::StreamReadReady(payload) => {
                State::with(|s| {
                    let reader = s.stream_readers.get_mut(&payload.stream).unwrap();
                    let (status, buffer) = assert_ready(pin!(
                        reader.read(Vec::with_capacity(payload.op_count as usize))
                    ));
                    assert_eq!(status, StreamResult::Complete(payload.ready_count as usize));
                    assert_eq!(buffer, stream_payload(payload.item, payload.ready_count));
                });
            }
            Command::StreamReadDropped(payload) => {
                State::with(|s| {
                    let reader = s.stream_readers.get_mut(&payload.stream).unwrap();
                    let (status, buffer) = assert_ready(pin!(
                        reader.read(Vec::with_capacity(payload.count as usize))
                    ));
                    assert_eq!(status, StreamResult::Dropped);
                    assert!(buffer.is_empty());
                });
            }
            Command::StreamWritePending(payload) => {
                debug!("write pending: {}", payload.stream);
                let mut writer = State::with(|s| s.stream_writers.remove(&payload.stream).unwrap());
                let (tx, rx) = oneshot::channel();
                State::with(|s| {
                    let prev = s.stream_write_cancel_signals.insert(payload.stream, tx);
                    assert!(prev.is_none());
                });
                let mut future = Box::pin(async move {
                    debug!("write pending start: {}", payload.stream);
                    let (result, remaining) = CancellableStreamWrite {
                        cancel: rx,
                        write: writer.write(stream_payload(payload.item, payload.count)),
                    }
                    .await;
                    debug!("write pending done: {} {result:?}", payload.stream);
                    State::with(|s| {
                        let _ = s.stream_write_cancel_signals.remove(&payload.stream);
                        match result {
                            StreamResult::Complete(n) => {
                                s.stream_writes_completed
                                    .insert(payload.stream, Ok((n, remaining)));
                            }
                            StreamResult::Dropped => {
                                s.stream_writes_completed
                                    .insert(payload.stream, Err((0, remaining)));
                            }
                            StreamResult::Cancelled => {}
                        }
                        let prev = s.stream_writers.insert(payload.stream, writer);
                        assert!(prev.is_none());
                    });
                });
                assert_not_ready(future.as_mut());
                wit_bindgen::spawn(future);

                pin_project! {
                    struct CancellableStreamWrite<'a> {
                        #[pin]
                        cancel: oneshot::Receiver<()>,
                        #[pin]
                        write: wit_bindgen::StreamWrite<'a, u32>,
                    }
                }

                impl Future for CancellableStreamWrite<'_> {
                    type Output = (StreamResult, Vec<u32>);

                    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
                        let this = self.project();
                        let (result, buffer) = match this.cancel.poll(cx) {
                            Poll::Ready(_) => this.write.cancel(),
                            Poll::Pending => match this.write.poll(cx) {
                                Poll::Ready(result) => result,
                                Poll::Pending => return Poll::Pending,
                            },
                        };
                        Poll::Ready((result, buffer.into_vec()))
                    }
                }
            }
            Command::StreamReadPending(payload) => {
                let mut reader = State::with(|s| s.stream_readers.remove(&payload.stream).unwrap());
                let (tx, rx) = oneshot::channel();
                State::with(|s| {
                    let prev = s.stream_read_cancel_signals.insert(payload.stream, tx);
                    assert!(prev.is_none());
                });
                let mut future = Box::pin(async move {
                    let (result, buf) = CancellableStreamRead {
                        cancel: rx,
                        read: reader.read(Vec::with_capacity(payload.count as usize)),
                    }
                    .await;
                    State::with(|s| {
                        let _ = s.stream_read_cancel_signals.remove(&payload.stream);
                        match result {
                            StreamResult::Complete(_) => {
                                s.stream_reads_completed.insert(payload.stream, Some(buf));
                            }
                            StreamResult::Dropped => {
                                assert!(buf.is_empty(), "dropped but got {}", buf.len());
                                s.stream_reads_completed.insert(payload.stream, None);
                            }
                            StreamResult::Cancelled => {}
                        }
                        let prev = s.stream_readers.insert(payload.stream, reader);
                        assert!(prev.is_none());
                    });
                });
                assert_not_ready(future.as_mut());
                wit_bindgen::spawn(future);

                pin_project! {
                    struct CancellableStreamRead<'a> {
                        #[pin]
                        cancel: oneshot::Receiver<()>,
                        #[pin]
                        read: wit_bindgen::StreamRead<'a, u32>,
                    }
                }

                impl Future for CancellableStreamRead<'_> {
                    type Output = (StreamResult, Vec<u32>);

                    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
                        let this = self.project();
                        let (result, buffer) = match this.cancel.poll(cx) {
                            Poll::Ready(_) => this.read.cancel(),
                            Poll::Pending => match this.read.poll(cx) {
                                Poll::Ready(result) => result,
                                Poll::Pending => return Poll::Pending,
                            },
                        };
                        Poll::Ready((result, buffer))
                    }
                }
            }
            Command::StreamCancelWrite(id) => {
                State::with(|s| {
                    s.stream_write_cancel_signals
                        .remove(&id)
                        .unwrap()
                        .send(())
                        .unwrap();
                });
                assert!(
                    State::test_property(|s| s.stream_writers.contains_key(&id)).await,
                    "expected cancel write {id} to be cancelled",
                );
            }
            Command::StreamCancelRead(id) => {
                State::with(|s| {
                    s.stream_read_cancel_signals
                        .remove(&id)
                        .unwrap()
                        .send(())
                        .unwrap();
                });
                assert!(
                    State::test_property(|s| s.stream_readers.contains_key(&id)).await,
                    "expected future read {id} to be cancelled",
                );
            }
            Command::StreamWriteAssertComplete(payload) => {
                assert!(
                    State::test_property(|s| {
                        match s.stream_writes_completed.remove(&payload.stream) {
                            Some(Ok((size, _buf))) => {
                                assert_eq!(size, payload.count as usize);
                                true
                            }
                            Some(Err(_)) => panic!("stream was dropped"),
                            None => false,
                        }
                    })
                    .await,
                    "expected stream write {} to be complete",
                    payload.stream,
                );
            }
            Command::StreamWriteAssertDropped(payload) => {
                assert!(
                    State::test_property(|s| {
                        match s.stream_writes_completed.remove(&payload.stream) {
                            Some(Err((size, _buf))) => {
                                assert_eq!(size, payload.count as usize);
                                true
                            }
                            Some(Ok(_)) => panic!("stream was not dropped"),
                            None => false,
                        }
                    })
                    .await,
                    "expected stream write {} to be complete",
                    payload.stream,
                );
            }
            Command::StreamReadAssertComplete(payload) => {
                assert!(
                    State::test_property(|s| {
                        match s.stream_reads_completed.remove(&payload.stream) {
                            Some(Some(i)) => {
                                assert_eq!(i, stream_payload(payload.item, payload.count));
                                true
                            }
                            Some(None) => panic!("stream was dropped"),
                            None => false,
                        }
                    })
                    .await,
                    "expected stream read {} to be complete",
                    payload.stream,
                );
            }
            Command::StreamReadAssertDropped(id) => {
                assert!(
                    State::test_property(|s| {
                        match s.stream_reads_completed.remove(&id) {
                            Some(None) => true,
                            Some(Some(_)) => panic!("stream was not dropped"),
                            None => false,
                        }
                    })
                    .await,
                    "expected stream read {id} to be complete",
                );
            }

            Command::Ack => {}
        }
    }
}

fn stream_payload(init: u32, count: u32) -> Vec<u32> {
    (init..init + count).collect()
}

fn assert_ready<F: Future>(f: Pin<&mut F>) -> F::Output {
    let mut cx = Context::from_waker(Waker::noop());
    match f.poll(&mut cx) {
        Poll::Ready(i) => i,
        Poll::Pending => panic!("future was pending"),
    }
}

fn assert_not_ready<F: Future>(f: Pin<&mut F>) {
    let mut cx = Context::from_waker(Waker::noop());
    match f.poll(&mut cx) {
        Poll::Ready(_) => panic!("future is ready"),
        Poll::Pending => {}
    }
}

fn main() {
    unreachable!();
}
