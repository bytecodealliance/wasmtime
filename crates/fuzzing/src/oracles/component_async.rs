//! For a high-level overview of this fuzz target see `fuzz_async.rs`

use crate::block_on;
use crate::generators::component_async::exports::wasmtime_fuzz::fuzz::async_test::Guest;
use crate::generators::component_async::wasmtime_fuzz::fuzz::async_test::{self, Command};
use crate::generators::component_async::wasmtime_fuzz::fuzz::types;
use crate::generators::component_async::{ComponentAsync, FuzzAsyncPre, Scope};
use futures::channel::oneshot;
use std::collections::{HashMap, HashSet};
use std::mem;
use std::pin::Pin;
use std::sync::{Arc, OnceLock, Weak};
use std::task::{Context, Poll, Waker};
use std::time::Instant;
use wasmtime::component::{
    Access, Accessor, AccessorTask, Component, Destination, FutureConsumer, FutureProducer,
    FutureReader, HasSelf, Linker, ResourceTable, Source, StreamConsumer, StreamProducer,
    StreamReader, StreamResult, VecBuffer,
};
use wasmtime::{AsContextMut, Config, Engine, Result, Store, StoreContextMut};
use wasmtime_wasi::{WasiCtx, WasiCtxView, WasiView};

static STATE: OnceLock<(Engine, FuzzAsyncPre<Data>)> = OnceLock::new();

/// Initializes state for future fuzz runs.
///
/// This will create an `Engine` to run this fuzzer within and it will
/// additionally precompile the component that will be used for fuzzing.
///
/// There are a few points of note about this:
///
/// * The `misc` fuzzer is manually instrumented with this function as the init
///   hook to ensure this runs before any other fuzzing.
///
/// * Compilation of the component takes quite some time with
///   fuzzing-instrumented Cranelift. To assist with local development this
///   implements a cache which is serialized/deserialized via an env var.
pub fn init() {
    crate::init_fuzzing();

    STATE.get_or_init(|| {
        let mut config = Config::new();
        config.wasm_component_model_async(true);
        config.async_support(true);
        let engine = Engine::new(&config).unwrap();
        let component = compile(&engine);
        let mut linker = Linker::new(&engine);
        wasmtime_wasi::p2::add_to_linker_async(&mut linker).unwrap();
        async_test::add_to_linker::<_, HasSelf<Data>>(&mut linker, |d| d).unwrap();
        types::add_to_linker::<_, HasSelf<Data>>(&mut linker, |d| d).unwrap();

        let pre = linker.instantiate_pre(&component).unwrap();
        let pre = FuzzAsyncPre::new(pre).unwrap();

        (engine, pre)
    });

    fn compile(engine: &Engine) -> Component {
        let wasm = test_programs_artifacts::FUZZ_ASYNC_COMPONENT;
        let cwasm_cache = std::env::var("COMPONENT_ASYNC_CWASM_CACHE").ok();
        if let Some(path) = &cwasm_cache
            && let Ok(cwasm_mtime) = std::fs::metadata(&path).and_then(|m| m.modified())
            && let Ok(wasm_mtime) = std::fs::metadata(wasm).and_then(|m| m.modified())
            && cwasm_mtime > wasm_mtime
        {
            log::debug!("Using cached component async cwasm at {path}");
            unsafe {
                return Component::deserialize_file(engine, path).unwrap();
            }
        }

        let composition = {
            let mut config = wasm_compose::config::Config::default();
            let tempdir = tempfile::TempDir::new().unwrap();
            let path = tempdir.path().join("fuzz-async.wasm");
            std::fs::copy(wasm, &path).unwrap();
            config.definitions.push(path.clone());

            wasm_compose::composer::ComponentComposer::new(&path, &config)
                .compose()
                .unwrap()
        };
        let start = Instant::now();
        let component = Component::new(&engine, &composition).unwrap();
        if let Some(path) = cwasm_cache {
            log::debug!("Caching component async cwasm to {path}");
            std::fs::write(path, &component.serialize().unwrap()).unwrap();
        } else if start.elapsed() > std::time::Duration::from_secs(1) {
            eprintln!(
                "
!!!!!!!!!!!!!!!!!!!!!!!!!!

Component compilation is slow, try setting `COMPONENT_ASYNC_CWASM_CACHE=path` to
cache compilation results

!!!!!!!!!!!!!!!!!!!!!!!!!!
"
            );
        }
        return component;
    }
}

#[derive(Default)]
struct Data {
    ctx: WasiCtx,
    table: ResourceTable,
    wakers: HashMap<Scope, Waker>,
    commands: Vec<(Scope, Command)>,

    guest_caller_stream: Option<StreamReader<Command>>,
    guest_callee_stream: Option<StreamReader<Command>>,

    host_pending_async_calls: HashMap<u32, oneshot::Sender<()>>,
    host_pending_async_calls_cancelled: HashSet<u32>,
    guest_pending_async_calls_ready: HashSet<u32>,

    // State of futures/streams. Note that while #12091 is unresolved an
    // `Arc`/`Weak` combo is used to detect when wasmtime drops futures/streams
    // and the various halves we're interacting with using traits.
    host_futures: HashMap<u32, FutureReader<u32>>,
    host_future_producers: HashMap<u32, (HostFutureProducerState, Weak<()>)>,
    host_future_consumers: HashMap<u32, (HostFutureConsumerState, Weak<()>)>,
    host_streams: HashMap<u32, StreamReader<u32>>,
    host_stream_producers: HashMap<u32, (HostStreamProducerState, Weak<()>)>,
    host_stream_consumers: HashMap<u32, (HostStreamConsumerState, Weak<()>)>,
}

impl WasiView for Data {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.ctx,
            table: &mut self.table,
        }
    }
}

impl async_test::HostWithStore for HasSelf<Data> {
    async fn async_ready<T>(_store: &Accessor<T, Self>) {}

    async fn async_pending<T>(store: &Accessor<T, Self>, id: u32) {
        let (tx, rx) = oneshot::channel();
        store.with(|mut s| s.get().host_pending_async_calls.insert(id, tx));
        let record = RecordCancelOnDrop { store, id };
        rx.await.unwrap();
        mem::forget(record);

        struct RecordCancelOnDrop<'a, T: 'static> {
            store: &'a Accessor<T, HasSelf<Data>>,
            id: u32,
        }

        impl<T> Drop for RecordCancelOnDrop<'_, T> {
            fn drop(&mut self) {
                self.store.with(|mut s| {
                    s.get().host_pending_async_calls_cancelled.insert(self.id);
                });
            }
        }
    }

    async fn init<T>(_store: &Accessor<T, Self>, _scope: types::Scope) {}
}

impl async_test::Host for Data {
    fn sync_ready(&mut self) {}

    fn future_take(&mut self, id: u32) -> FutureReader<u32> {
        self.host_futures.remove(&id).unwrap()
    }

    fn future_receive(&mut self, id: u32, future: FutureReader<u32>) {
        let prev = self.host_futures.insert(id, future);
        assert!(prev.is_none());
    }

    fn stream_take(&mut self, id: u32) -> StreamReader<u32> {
        self.host_streams.remove(&id).unwrap()
    }

    fn stream_receive(&mut self, id: u32, stream: StreamReader<u32>) {
        let prev = self.host_streams.insert(id, stream);
        assert!(prev.is_none());
    }
}

impl types::HostWithStore for HasSelf<Data> {
    fn get_commands<T>(
        mut store: Access<'_, T, Self>,
        scope: types::Scope,
    ) -> StreamReader<Command> {
        let data = store.get();
        match scope {
            types::Scope::Caller => data.guest_caller_stream.take().unwrap(),
            types::Scope::Callee => data.guest_callee_stream.take().unwrap(),
        }
    }
}

impl types::Host for Data {}

/// Executes the `input` provided, assuming that `init` has been previously
/// executed.
pub fn run(mut input: ComponentAsync) {
    log::debug!("Running component async fuzz test with\n{input:?}");
    input.commands.reverse();
    let (engine, pre) = STATE.get().unwrap();
    let mut store = Store::new(
        engine,
        Data {
            ctx: WasiCtx::builder().inherit_stdio().inherit_env().build(),
            commands: input.commands,
            ..Data::default()
        },
    );

    let guest_caller_stream = StreamReader::new(&mut store, SharedStream(Scope::GuestCaller));
    let guest_callee_stream = StreamReader::new(&mut store, SharedStream(Scope::GuestCallee));
    store.data_mut().guest_caller_stream = Some(guest_caller_stream);
    store.data_mut().guest_callee_stream = Some(guest_callee_stream);
    block_on(async {
        let instance = pre.instantiate_async(&mut store).await.unwrap();
        let test = instance.wasmtime_fuzz_fuzz_async_test();

        let mut host_caller = SharedStream(Scope::HostCaller);
        let mut host_callee = SharedStream(Scope::HostCallee);
        store
            .run_concurrent(async |store| {
                // Kick off stream reads in the guest. This function will return
                // but the tasks in the guest will keep running after they
                // return to process stream items.
                test.call_init(store, types::Scope::Caller).await.unwrap();

                // Simultaneously process commands from both host streams. These
                // will return once the entire command queue is exhausted.
                futures::join!(
                    async {
                        while let Some(cmd) = host_caller.next(store).await {
                            host_caller_cmd(&test, store, cmd).await;
                        }
                    },
                    async {
                        while let Some(cmd) = host_callee.next(store).await {
                            host_callee_cmd(store, cmd).await;
                        }
                    },
                );

                // Note that there may still be pending async work in the guest
                // (or host). It's intentional that it's not cleaned up here to
                // help test situations where async work is all abruptly
                // cancelled by just being dropped in the host.
            })
            .await
            .unwrap();
    });
}

/// See documentation in `fuzz_async.rs` for what's going on here.
async fn test_property<F>(store: &Accessor<Data>, mut f: F) -> bool
where
    F: FnMut(&mut Data) -> bool,
{
    for _ in 0..1000 {
        let ready = store.with(|mut s| f(s.get()));
        if ready {
            return true;
        }

        crate::YieldN(1).await;
    }

    return false;
}

async fn await_property<F>(store: &Accessor<Data>, desc: &str, f: F)
where
    F: FnMut(&mut Data) -> bool,
{
    assert!(
        test_property(store, f).await,
        "timed out waiting for {desc}",
    );
}

async fn host_caller_cmd(test: &Guest, store: &Accessor<Data>, cmd: Command) {
    match cmd {
        Command::Ack => {}
        Command::SyncReadyCall => test.call_sync_ready(store).await.unwrap(),
        Command::AsyncReadyCall => test.call_async_ready(store).await.unwrap(),
        Command::AsyncPendingExportComplete(_i) => todo!(),
        Command::AsyncPendingExportAssertCancelled(_i) => todo!(),
        Command::AsyncPendingImportCall(i) => {
            struct RunPendingImport {
                test: Guest,
                i: u32,
            }

            store.spawn(RunPendingImport {
                test: test.clone(),
                i,
            });

            impl AccessorTask<Data> for RunPendingImport {
                async fn run(self, store: &Accessor<Data>) -> Result<()> {
                    self.test.call_async_pending(store, self.i).await?;
                    store.with(|mut s| {
                        s.get().guest_pending_async_calls_ready.insert(self.i);
                    });
                    Ok(())
                }
            }
        }
        Command::AsyncPendingImportCancel(_i) => todo!(),
        Command::AsyncPendingImportAssertReady(i) => {
            assert!(
                test_property(store, |s| s.guest_pending_async_calls_ready.remove(&i)).await,
                "expected async_pending import {i} to be ready",
            );
        }

        Command::FutureTake(i) => {
            let future = test.call_future_take(store, i).await.unwrap();
            store.with(|mut s| {
                let prev = s.get().host_futures.insert(i, future);
                assert!(prev.is_none());
            });
        }
        Command::FutureGive(i) => {
            let future = store.with(|mut s| s.get().host_futures.remove(&i).unwrap());
            test.call_future_receive(store, i, future).await.unwrap();
        }
        Command::StreamTake(i) => {
            let stream = test.call_stream_take(store, i).await.unwrap();
            store.with(|mut s| {
                let prev = s.get().host_streams.insert(i, stream);
                assert!(prev.is_none());
            });
        }
        Command::StreamGive(i) => {
            let stream = store.with(|mut s| s.get().host_streams.remove(&i).unwrap());
            test.call_stream_receive(store, i, stream).await.unwrap();
        }

        other => future_or_stream_cmd(store, other).await,
    }
}

async fn host_callee_cmd(store: &Accessor<Data>, cmd: Command) {
    match cmd {
        Command::Ack => {}
        Command::SyncReadyCall => todo!(),
        Command::AsyncReadyCall => todo!(),
        Command::AsyncPendingExportComplete(i) => store.with(|mut s| {
            s.get()
                .host_pending_async_calls
                .remove(&i)
                .unwrap()
                .send(())
                .unwrap();
        }),
        Command::AsyncPendingExportAssertCancelled(i) => {
            assert!(
                test_property(store, |s| s.host_pending_async_calls_cancelled.remove(&i)).await,
                "expected async_pending export {i} to be cancelled",
            );
        }
        Command::AsyncPendingImportCall(_i) => todo!(),
        Command::AsyncPendingImportCancel(_i) => todo!(),
        Command::AsyncPendingImportAssertReady(_i) => todo!(),

        other => future_or_stream_cmd(store, other).await,
    }
}

async fn future_or_stream_cmd(store: &Accessor<Data>, cmd: Command) {
    match cmd {
        // These commands should be handled above
        Command::Ack
        | Command::SyncReadyCall
        | Command::AsyncReadyCall
        | Command::AsyncPendingExportComplete(_)
        | Command::AsyncPendingExportAssertCancelled(_)
        | Command::AsyncPendingImportCall(_)
        | Command::AsyncPendingImportCancel(_)
        | Command::FutureTake(_)
        | Command::FutureGive(_)
        | Command::StreamTake(_)
        | Command::StreamGive(_)
        | Command::AsyncPendingImportAssertReady(_) => unreachable!(),

        Command::FutureNew(id) => {
            store.with(|mut s| {
                let arc = Arc::new(());
                let weak = Arc::downgrade(&arc);
                let future = FutureReader::new(&mut s, HostFutureProducer(id, arc));
                let data = s.get();
                let prev = data.host_futures.insert(id, future);
                assert!(prev.is_none());
                let prev = data
                    .host_future_producers
                    .insert(id, (HostFutureProducerState::Idle, weak));
                assert!(prev.is_none());
            });
        }
        Command::FutureDropReadable(id) => {
            store.with(|mut s| match s.get().host_futures.remove(&id) {
                Some(mut future) => future.close(&mut s),
                None => {
                    let (mut state, _weak) = s.get().host_future_consumers.remove(&id).unwrap();
                    state.wake_by_ref();
                }
            })
        }
        Command::FutureWriteReady(payload) => {
            await_property(store, "future write should be waiting", |s| {
                matches!(
                    s.host_future_producers.get(&payload.future),
                    Some((HostFutureProducerState::Waiting(_), _))
                )
            })
            .await;
            store.with(|mut s| {
                let state = s
                    .get()
                    .host_future_producers
                    .get_mut(&payload.future)
                    .unwrap();
                match state {
                    (HostFutureProducerState::Waiting(waker), _) => {
                        waker.wake_by_ref();
                        state.0 = HostFutureProducerState::Writing(payload.item);
                    }
                    (state, _) => panic!("future not waiting: {state:?}"),
                }
            })
        }
        Command::FutureWritePending(payload) => store.with(|mut s| {
            let state = s
                .get()
                .host_future_producers
                .get_mut(&payload.future)
                .unwrap();
            match state {
                (HostFutureProducerState::Idle, _) => {
                    state.0 = HostFutureProducerState::Writing(payload.item);
                }
                _ => panic!("future not idle"),
            }
        }),
        Command::FutureWriteDropped(id) => store.with(|mut s| {
            let (state, weak) = s.get().host_future_producers.remove(&id).unwrap();
            assert!(matches!(state, HostFutureProducerState::Idle));
            assert!(weak.upgrade().is_none());
        }),
        Command::FutureReadReady(payload) => {
            let id = payload.future;
            store.with(|mut s| {
                let arc = Arc::new(());
                let weak = Arc::downgrade(&arc);
                let data = s.get();
                let future = data.host_futures.remove(&id).unwrap();
                let prev = data
                    .host_future_consumers
                    .insert(id, (HostFutureConsumerState::Consuming, weak));
                assert!(prev.is_none());
                future.pipe(&mut s, HostFutureConsumer(id, arc));
            });

            await_property(store, "future should be present", |s| {
                matches!(
                    s.host_future_consumers[&id],
                    (HostFutureConsumerState::Complete(_), _)
                )
            })
            .await;

            store.with(|mut s| {
                let (state, _) = s.get().host_future_consumers.remove(&id).unwrap();
                match state {
                    HostFutureConsumerState::Complete(i) => assert_eq!(i, payload.item),
                    _ => panic!("future not complete"),
                }
            });
        }
        Command::FutureReadPending(id) => {
            ensure_future_reading(store, id);
            store.with(|mut s| {
                let (state, _) = s.get().host_future_consumers.get_mut(&id).unwrap();
                state.wake_by_ref();
                assert!(
                    matches!(state, HostFutureConsumerState::Idle),
                    "bad state: {state:?}",
                );
                *state = HostFutureConsumerState::Consuming;
            })
        }
        Command::FutureCancelWrite(id) => store.with(|mut s| {
            let (state, _) = s.get().host_future_producers.get_mut(&id).unwrap();
            assert!(matches!(state, HostFutureProducerState::Writing(_)));
            *state = HostFutureProducerState::Idle;
        }),
        Command::FutureCancelRead(id) => store.with(|mut s| {
            let (state, _) = s.get().host_future_consumers.get_mut(&id).unwrap();
            assert!(matches!(state, HostFutureConsumerState::Consuming));
            *state = HostFutureConsumerState::Idle;
        }),
        Command::FutureReadAssertComplete(payload) => {
            await_property(store, "future read should be complete", |s| {
                matches!(
                    s.host_future_consumers.get(&payload.future),
                    Some((HostFutureConsumerState::Complete(_), _))
                )
            })
            .await;
            store.with(|mut s| {
                let (state, _) = s
                    .get()
                    .host_future_consumers
                    .remove(&payload.future)
                    .unwrap();
                match state {
                    HostFutureConsumerState::Complete(i) => assert_eq!(i, payload.item),
                    _ => panic!("future not complete"),
                }
            })
        }
        Command::FutureWriteAssertComplete(id) => store.with(|mut s| {
            let (state, weak) = s.get().host_future_producers.remove(&id).unwrap();
            assert!(matches!(state, HostFutureProducerState::Complete));
            assert!(weak.upgrade().is_none());
        }),
        Command::FutureWriteAssertDropped(id) => store.with(|mut s| {
            let (state, weak) = s.get().host_future_producers.remove(&id).unwrap();
            assert!(matches!(state, HostFutureProducerState::Writing(_)));
            assert!(weak.upgrade().is_none());
        }),

        Command::StreamNew(id) => {
            store.with(|mut s| {
                let arc = Arc::new(());
                let weak = Arc::downgrade(&arc);
                let stream = StreamReader::new(&mut s, HostStreamProducer(id, arc));
                let data = s.get();
                let prev = data.host_streams.insert(id, stream);
                assert!(prev.is_none());
                let prev = data
                    .host_stream_producers
                    .insert(id, (HostStreamProducerState::idle(), weak));
                assert!(prev.is_none());
            });
        }
        Command::StreamDropReadable(id) => {
            store.with(|mut s| match s.get().host_streams.remove(&id) {
                Some(mut stream) => {
                    stream.close(&mut s);
                }
                None => {
                    let (mut state, _weak) = s.get().host_stream_consumers.remove(&id).unwrap();
                    state.wake_by_ref();
                }
            })
        }
        Command::StreamDropWritable(id) => store.with(|mut s| {
            let (mut state, _weak) = s.get().host_stream_producers.remove(&id).unwrap();
            state.wake_by_ref();
        }),
        Command::StreamWriteReady(payload) => {
            let id = payload.stream;
            store.with(|mut s| {
                let (state, _) = s.get().host_stream_producers.get_mut(&id).unwrap();
                state.wake_by_ref();
                match state.kind {
                    HostStreamProducerStateKind::Idle => {
                        state.kind = HostStreamProducerStateKind::Writing(stream_payload(
                            payload.item,
                            payload.op_count,
                        ));
                    }
                    _ => panic!("stream not idle: {state:?}"),
                }
            });
            await_property(store, "stream should complete a write", |s| {
                matches!(
                    s.host_stream_producers[&id].0.kind,
                    HostStreamProducerStateKind::Wrote(_),
                )
            })
            .await;
            store.with(|mut s| {
                let (state, _) = s.get().host_stream_producers.get_mut(&id).unwrap();
                match state.kind {
                    HostStreamProducerStateKind::Wrote(amt) => {
                        assert_eq!(amt, payload.ready_count);
                        state.kind = HostStreamProducerStateKind::Idle;
                    }
                    _ => panic!("stream not idle: {state:?}"),
                }
            });
        }
        Command::StreamReadReady(payload) => {
            let id = payload.stream;
            ensure_stream_reading(store, id);
            store.with(|mut s| {
                let (state, _) = s.get().host_stream_consumers.get_mut(&id).unwrap();
                state.wake_by_ref();
                state.kind = HostStreamConsumerStateKind::Consuming(payload.op_count);
            });
            await_property(store, "stream should complete a read", |s| {
                matches!(
                    s.host_stream_consumers[&id].0.kind,
                    HostStreamConsumerStateKind::Consumed(_),
                )
            })
            .await;

            store.with(|mut s| {
                let (state, _) = s.get().host_stream_consumers.get_mut(&id).unwrap();
                match &state.kind {
                    HostStreamConsumerStateKind::Consumed(last_read) => {
                        assert_eq!(
                            *last_read,
                            stream_payload(payload.item, payload.ready_count)
                        );
                        state.kind = HostStreamConsumerStateKind::Idle;
                    }
                    _ => panic!("future not complete"),
                }
            });
        }
        Command::StreamWritePending(payload) => store.with(|mut s| {
            let (state, _) = s
                .get()
                .host_stream_producers
                .get_mut(&payload.stream)
                .unwrap();
            state.wake_by_ref();
            match state.kind {
                HostStreamProducerStateKind::Idle => {
                    state.kind = HostStreamProducerStateKind::Writing(stream_payload(
                        payload.item,
                        payload.count,
                    ));
                }
                _ => panic!("stream not idle {:?}", state.kind),
            }
        }),
        Command::StreamReadPending(payload) => {
            ensure_stream_reading(store, payload.stream);
            store.with(|mut s| {
                let (state, _) = s
                    .get()
                    .host_stream_consumers
                    .get_mut(&payload.stream)
                    .unwrap();
                state.wake_by_ref();
                assert!(matches!(state.kind, HostStreamConsumerStateKind::Idle));
                state.kind = HostStreamConsumerStateKind::Consuming(payload.count);
            })
        }
        Command::StreamWriteDropped(payload) => store.with(|mut s| {
            let (state, weak) = s
                .get()
                .host_stream_producers
                .get_mut(&payload.stream)
                .unwrap();
            assert!(matches!(state.kind, HostStreamProducerStateKind::Idle));
            assert!(weak.upgrade().is_none());
        }),
        Command::StreamReadDropped(payload) => {
            ensure_stream_reading(store, payload.stream);
            await_property(store, "stream read should get dropped", |s| {
                let weak = &s.host_stream_consumers[&payload.stream].1;
                weak.upgrade().is_none()
            })
            .await;
            store.with(|mut s| {
                let (state, weak) = s
                    .get()
                    .host_stream_consumers
                    .get_mut(&payload.stream)
                    .unwrap();
                assert!(matches!(state.kind, HostStreamConsumerStateKind::Idle));
                assert!(weak.upgrade().is_none());
            })
        }
        Command::StreamCancelWrite(id) => store.with(|mut s| {
            let (state, _) = s.get().host_stream_producers.get_mut(&id).unwrap();
            assert!(
                matches!(state.kind, HostStreamProducerStateKind::Writing(_)),
                "invalid state {state:?}",
            );
            state.kind = HostStreamProducerStateKind::Idle;
            state.wake_by_ref();
        }),
        Command::StreamCancelRead(id) => store.with(|mut s| {
            let (state, _) = s.get().host_stream_consumers.get_mut(&id).unwrap();
            assert!(matches!(
                state.kind,
                HostStreamConsumerStateKind::Consuming(_)
            ));
            state.kind = HostStreamConsumerStateKind::Idle;
        }),
        Command::StreamReadAssertComplete(payload) => store.with(|mut s| {
            let (state, _) = s
                .get()
                .host_stream_consumers
                .get_mut(&payload.stream)
                .unwrap();
            match &state.kind {
                HostStreamConsumerStateKind::Consumed(last_read) => {
                    assert_eq!(*last_read, stream_payload(payload.item, payload.count));
                    state.kind = HostStreamConsumerStateKind::Idle;
                }
                _ => panic!("stream not complete"),
            }
        }),
        Command::StreamWriteAssertComplete(payload) => store.with(|mut s| {
            let (state, _) = s
                .get()
                .host_stream_producers
                .get_mut(&payload.stream)
                .unwrap();
            match state.kind {
                HostStreamProducerStateKind::Wrote(amt) => {
                    assert_eq!(amt, payload.count);
                    state.kind = HostStreamProducerStateKind::Idle;
                }
                _ => panic!("stream not complete: {:?}", state.kind),
            }
        }),
        Command::StreamWriteAssertDropped(payload) => {
            await_property(store, "stream write should be dropped", |s| {
                let weak = &s.host_stream_producers[&payload.stream].1;
                weak.upgrade().is_none()
            })
            .await;
            store.with(|mut s| {
                let (state, weak) = s
                    .get()
                    .host_stream_producers
                    .get_mut(&payload.stream)
                    .unwrap();
                assert!(matches!(
                    state.kind,
                    HostStreamProducerStateKind::Writing(_)
                ));
                assert!(weak.upgrade().is_none());
            })
        }
        Command::StreamReadAssertDropped(id) => {
            await_property(store, "stream read should be dropped", |s| {
                let weak = &s.host_stream_consumers[&id].1;
                weak.upgrade().is_none()
            })
            .await;
            store.with(|mut s| {
                let (state, weak) = s.get().host_stream_consumers.get_mut(&id).unwrap();
                assert!(matches!(
                    state.kind,
                    HostStreamConsumerStateKind::Consuming(_),
                ));
                assert!(weak.upgrade().is_none());
            })
        }
    }
}

fn stream_payload(item: u32, count: u32) -> Vec<u32> {
    (item..item + count).collect()
}

fn ensure_future_reading(store: &Accessor<Data>, id: u32) {
    store.with(|mut s| {
        let data = s.get();
        if !data.host_futures.contains_key(&id) {
            return;
        }
        log::debug!("future consume: start {id}");
        let arc = Arc::new(());
        let weak = Arc::downgrade(&arc);
        let data = s.get();
        let future = data.host_futures.remove(&id).unwrap();
        let prev = data
            .host_future_consumers
            .insert(id, (HostFutureConsumerState::Idle, weak));
        assert!(prev.is_none());
        future.pipe(&mut s, HostFutureConsumer(id, arc));
    });
}

fn ensure_stream_reading(store: &Accessor<Data>, id: u32) {
    store.with(|mut s| {
        let data = s.get();
        if !data.host_streams.contains_key(&id) {
            return;
        }
        log::debug!("stream consume: start {id}");
        let arc = Arc::new(());
        let weak = Arc::downgrade(&arc);
        let prev = data.host_stream_consumers.insert(
            id,
            (
                HostStreamConsumerState {
                    kind: HostStreamConsumerStateKind::Idle,
                    waker: None,
                },
                weak,
            ),
        );
        assert!(prev.is_none());
        let stream = data.host_streams.remove(&id).unwrap();
        stream.pipe(&mut s, HostStreamConsumer(id, arc));
    });
}

struct HostFutureConsumer(u32, #[expect(dead_code, reason = "drop-tracking")] Arc<()>);

/// Note that this is only created once a read is actually initiated on a
/// future. It's also not possible to cancel a host-based read on a future,
/// hence why this is simpler than the `HostFutureProducerState` state below.
#[derive(Debug)]
enum HostFutureConsumerState {
    Idle,
    Waiting(Waker),
    Consuming,
    Complete(u32),
}

impl HostFutureConsumerState {
    fn wake_by_ref(&mut self) {
        if let HostFutureConsumerState::Waiting(waker) = &self {
            waker.wake_by_ref();
            *self = HostFutureConsumerState::Idle;
        }
    }
}

impl FutureConsumer<Data> for HostFutureConsumer {
    type Item = u32;

    fn poll_consume(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        mut store: StoreContextMut<'_, Data>,
        mut source: Source<'_, Self::Item>,
        finish: bool,
    ) -> Poll<Result<()>> {
        let state = match store.data_mut().host_future_consumers.get_mut(&self.0) {
            Some(state) => state,
            None => {
                log::debug!("consume: closed {}", self.0);
                return Poll::Ready(Ok(()));
            }
        };
        match state.0 {
            HostFutureConsumerState::Idle | HostFutureConsumerState::Waiting(_) => {
                if finish {
                    log::debug!("consume: cancel {}", self.0);
                    state.0 = HostFutureConsumerState::Idle;
                    Poll::Ready(Ok(()))
                } else {
                    log::debug!("consume: wait {}", self.0);
                    state.0 = HostFutureConsumerState::Waiting(cx.waker().clone());
                    Poll::Pending
                }
            }
            HostFutureConsumerState::Consuming => {
                log::debug!("consume: done {}", self.0);
                let mut item = None;
                source.read(&mut store, &mut item).unwrap();
                store
                    .data_mut()
                    .host_future_consumers
                    .get_mut(&self.0)
                    .unwrap()
                    .0 = HostFutureConsumerState::Complete(item.unwrap());
                Poll::Ready(Ok(()))
            }
            HostFutureConsumerState::Complete(_) => unreachable!(),
        }
    }
}

struct HostFutureProducer(u32, #[expect(dead_code, reason = "drop-tracking")] Arc<()>);

#[derive(Debug)]
enum HostFutureProducerState {
    Idle,
    Waiting(Waker),
    Writing(u32),
    Complete,
}

impl FutureProducer<Data> for HostFutureProducer {
    type Item = u32;

    fn poll_produce(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        mut store: StoreContextMut<'_, Data>,
        finish: bool,
    ) -> Poll<Result<Option<Self::Item>>> {
        let state = store
            .data_mut()
            .host_future_producers
            .get_mut(&self.0)
            .unwrap();
        match state.0 {
            HostFutureProducerState::Idle | HostFutureProducerState::Waiting(_) => {
                if finish {
                    log::debug!("produce: cancel {}", self.0);
                    state.0 = HostFutureProducerState::Idle;
                    Poll::Ready(Ok(None))
                } else {
                    log::debug!("produce: wait {}", self.0);
                    state.0 = HostFutureProducerState::Waiting(cx.waker().clone());
                    Poll::Pending
                }
            }
            HostFutureProducerState::Writing(item) => {
                log::debug!("produce: done {}", self.0);
                state.0 = HostFutureProducerState::Complete;
                Poll::Ready(Ok(Some(item)))
            }
            HostFutureProducerState::Complete => unreachable!(),
        }
    }
}

struct HostStreamConsumer(u32, #[expect(dead_code, reason = "drop-tracking")] Arc<()>);

#[derive(Debug)]
struct HostStreamConsumerState {
    waker: Option<Waker>,
    kind: HostStreamConsumerStateKind,
}

#[derive(Debug)]
enum HostStreamConsumerStateKind {
    Idle,
    Consuming(u32),
    Consumed(Vec<u32>),
}

impl HostStreamConsumerState {
    fn wake_by_ref(&mut self) {
        if let Some(waker) = self.waker.take() {
            waker.wake();
        }
    }
}

impl StreamConsumer<Data> for HostStreamConsumer {
    type Item = u32;

    fn poll_consume(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        mut store: StoreContextMut<'_, Data>,
        mut source: Source<'_, Self::Item>,
        finish: bool,
    ) -> Poll<Result<StreamResult>> {
        let remaining = source.remaining(&mut store);
        let state = match store.data_mut().host_stream_consumers.get_mut(&self.0) {
            Some((state, _)) => state,
            None => {
                log::debug!("stream consume: dropped {}", self.0);
                return Poll::Ready(Ok(StreamResult::Dropped));
            }
        };
        match state.kind {
            HostStreamConsumerStateKind::Idle | HostStreamConsumerStateKind::Consumed(_) => {
                if finish {
                    log::debug!("stream consume: cancel {}", self.0);
                    state.waker = None;
                    Poll::Ready(Ok(StreamResult::Cancelled))
                } else {
                    log::debug!("stream consume: wait {}", self.0);
                    state.waker = Some(cx.waker().clone());
                    Poll::Pending
                }
            }
            HostStreamConsumerStateKind::Consuming(amt) => {
                // The writer is performing a zero-length write. We always
                // complete that without updating our own state.
                if remaining == 0 {
                    log::debug!("stream consume: completing zero-length write {}", self.0);
                    return Poll::Ready(Ok(StreamResult::Completed));
                }

                // If this is a zero-length read then block the writer but update our own state.
                if amt == 0 {
                    log::debug!("stream consume: finishing zero-length read {}", self.0);
                    state.kind = HostStreamConsumerStateKind::Consumed(Vec::new());
                    state.waker = Some(cx.waker().clone());
                    return Poll::Pending;
                }

                // For non-zero sizes perform the read/copy.
                log::debug!("stream consume: done {}", self.0);
                let mut dst = Vec::with_capacity(amt as usize);
                source.read(&mut store, &mut dst).unwrap();
                let state = &mut store
                    .data_mut()
                    .host_stream_consumers
                    .get_mut(&self.0)
                    .unwrap()
                    .0;
                state.kind = HostStreamConsumerStateKind::Consumed(dst);
                state.waker = None;
                Poll::Ready(Ok(StreamResult::Completed))
            }
        }
    }
}

impl Drop for HostStreamConsumer {
    fn drop(&mut self) {
        log::debug!("stream consume: drop {}", self.0);
    }
}

struct HostStreamProducer(u32, #[expect(dead_code, reason = "drop-tracking")] Arc<()>);

#[derive(Debug)]
struct HostStreamProducerState {
    kind: HostStreamProducerStateKind,
    waker: Option<Waker>,
}

#[derive(Debug)]
enum HostStreamProducerStateKind {
    Idle,
    Writing(Vec<u32>),
    Wrote(u32),
}

impl HostStreamProducerState {
    fn idle() -> Self {
        HostStreamProducerState {
            kind: HostStreamProducerStateKind::Idle,
            waker: None,
        }
    }

    fn wake_by_ref(&mut self) {
        if let Some(waker) = self.waker.take() {
            waker.wake();
        }
    }
}

impl StreamProducer<Data> for HostStreamProducer {
    type Item = u32;
    type Buffer = VecBuffer<u32>;

    fn poll_produce(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        mut store: StoreContextMut<'_, Data>,
        mut dst: Destination<'_, Self::Item, Self::Buffer>,
        finish: bool,
    ) -> Poll<Result<StreamResult>> {
        let remaining = dst.remaining(&mut store);
        let data = store.data_mut();
        let state = match data.host_stream_producers.get_mut(&self.0) {
            Some((state, _)) => state,
            None => {
                log::debug!("stream produce: dropped {}", self.0);
                return Poll::Ready(Ok(StreamResult::Dropped));
            }
        };
        match &mut state.kind {
            HostStreamProducerStateKind::Idle | HostStreamProducerStateKind::Wrote(_) => {
                if finish {
                    log::debug!("stream produce: cancel {}", self.0);
                    state.waker = None;
                    Poll::Ready(Ok(StreamResult::Cancelled))
                } else {
                    log::debug!("stream produce: wait {}", self.0);
                    state.waker = Some(cx.waker().clone());
                    Poll::Pending
                }
            }
            HostStreamProducerStateKind::Writing(buf) => {
                // Keep the other side blocked for a zero-length write
                // originated from the host.
                if buf.len() == 0 {
                    log::debug!("stream produce: zero-length write {}", self.0);
                    state.kind = HostStreamProducerStateKind::Wrote(0);
                    state.waker = Some(cx.waker().clone());
                    return Poll::Pending;
                }
                log::debug!("stream produce: write {}", self.0);
                match remaining {
                    Some(amt) => {
                        // If the guest is doing a zero-length read then we've
                        // got some data for them. Complete the read but leave
                        // ourselves in the same `Writing` state as before.
                        if amt == 0 {
                            state.waker = None;
                            return Poll::Ready(Ok(StreamResult::Completed));
                        }

                        // Don't let wasmtime buffer up data for us, so truncate
                        // the buffer we're sending over to the amount that the
                        // reader is requesting.
                        if amt < buf.len() {
                            buf.truncate(amt);
                        }
                    }

                    // At this time host<->host stream reads/writes aren't
                    // fuzzed since that brings up a bunch of weird edge cases
                    // which aren't fun to deal with and aren't interesting
                    // either.
                    None => unreachable!(),
                }
                let count = buf.len() as u32;
                dst.set_buffer(mem::take(buf).into());
                state.kind = HostStreamProducerStateKind::Wrote(count);
                state.waker = None;
                Poll::Ready(Ok(StreamResult::Completed))
            }
        }
    }
}

impl Drop for HostStreamProducer {
    fn drop(&mut self) {
        log::debug!("stream produce: drop {}", self.0);
    }
}

struct SharedStream(Scope);

impl SharedStream {
    async fn next(&mut self, accessor: &Accessor<Data>) -> Option<Command> {
        std::future::poll_fn(|cx| {
            accessor.with(|mut store| {
                self.poll(cx, store.as_context_mut(), false)
                    .map(|pair| match pair {
                        (None, StreamResult::Dropped) => None,
                        (Some(item), StreamResult::Completed) => Some(item),
                        _ => unreachable!(),
                    })
            })
        })
        .await
    }

    fn poll(
        &mut self,
        cx: &mut Context<'_>,
        mut store: StoreContextMut<'_, Data>,
        finish: bool,
    ) -> Poll<(Option<Command>, StreamResult)> {
        let data = store.data_mut();

        // If no more commands remain then this is a closed and dropped stream.
        let Some((scope, command)) = data.commands.last_mut() else {
            log::debug!("Stream closed: {:?}", self.0);
            return Poll::Ready((None, StreamResult::Dropped));
        };

        // If the next queued up command is for the scope that this stream is
        // attached to then send off the command.
        if *scope == self.0 {
            let ret = Some(*command);

            // All commands are followed up with an "ack", and after the "ack"
            // is delivered then the command is popped to move on to the next
            // command. The reason for this is to guarantee that a command has
            // been processed before moving on to the next command. This helps
            // make the fuzzing easier to work with by being able to implicitly
            // assume that a command has been processed by the time something
            // else is. Otherwise it might be possible that wasmtime has a set
            // of commands/callbacks that are all delivered at the same time and
            // the component model doesn't specify what order they happen
            // within. By forcing an "ack" it ensures a more expected ordering
            // of execution to assist with fuzzing without losing really all
            // that much coverage.
            if matches!(command, Command::Ack) {
                data.commands.pop();
            } else {
                *command = Command::Ack;
            }

            // After a command was popped other streams may be able to make
            // progress so wake them all up.
            for (_, waker) in data.wakers.drain() {
                waker.wake();
            }
            log::debug!("Delivering command {ret:?} for {:?}", self.0);
            return Poll::Ready((ret, StreamResult::Completed));
        }

        // The command queue is non-empty and the next command isn't meant for
        // us, so someone else needs to drain the queue. Enqueue our waker.
        if finish {
            Poll::Ready((None, StreamResult::Cancelled))
        } else {
            data.wakers.insert(self.0, cx.waker().clone());
            Poll::Pending
        }
    }
}

impl StreamProducer<Data> for SharedStream {
    type Item = Command;
    type Buffer = Option<Command>;

    fn poll_produce<'a>(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        store: StoreContextMut<'a, Data>,
        mut destination: Destination<'a, Self::Item, Self::Buffer>,
        finish: bool,
    ) -> Poll<Result<StreamResult>> {
        let (item, result) = std::task::ready!(self.poll(cx, store, finish));
        destination.set_buffer(item);
        Poll::Ready(Ok(result))
    }
}

#[cfg(test)]
mod tests {
    use super::{ComponentAsync, Scope, init, run};
    use crate::oracles::component_async::types::*;
    use crate::test::test_n_times;
    use Scope::*;

    #[test]
    fn smoke() {
        init();

        test_n_times(50, |c, _| {
            run(c);
            Ok(())
        });
    }

    // ========================================================================
    // A series of fuzz-generated test cases which caused problems during the
    // development of this fuzzer. Feel free to delete/edit/etc if the fuzzer
    // changes over time.

    #[test]
    fn simple() {
        init();

        run(ComponentAsync {
            commands: vec![
                (GuestCaller, Command::AsyncPendingImportCall(0)),
                (GuestCallee, Command::AsyncPendingImportCall(1)),
                (GuestCallee, Command::AsyncPendingExportComplete(0)),
                (GuestCaller, Command::AsyncPendingImportAssertReady(0)),
                (GuestCaller, Command::AsyncPendingImportCall(2)),
            ],
        });
    }

    #[test]
    fn somewhat_larger() {
        static COMMANDS: &[(Scope, Command)] = &[
            (GuestCallee, Command::FutureNew(0)),
            (HostCaller, Command::FutureNew(1)),
            (GuestCallee, Command::FutureReadPending(0)),
            (GuestCaller, Command::AsyncPendingImportCall(2)),
            (GuestCaller, Command::AsyncPendingImportCall(3)),
            (GuestCaller, Command::AsyncPendingImportCall(4)),
            (GuestCaller, Command::AsyncPendingImportCall(5)),
            (GuestCallee, Command::AsyncPendingExportComplete(5)),
            (GuestCallee, Command::AsyncPendingExportComplete(3)),
            (GuestCallee, Command::AsyncPendingExportComplete(4)),
            (GuestCallee, Command::AsyncPendingExportComplete(2)),
            (GuestCaller, Command::AsyncPendingImportCall(6)),
            (GuestCallee, Command::AsyncPendingExportComplete(6)),
            (GuestCaller, Command::AsyncPendingImportCall(7)),
            (GuestCallee, Command::AsyncPendingExportComplete(7)),
            (GuestCaller, Command::AsyncPendingImportCall(8)),
            (GuestCallee, Command::AsyncPendingExportComplete(8)),
            (GuestCaller, Command::AsyncPendingImportCall(9)),
            (GuestCallee, Command::AsyncPendingExportComplete(9)),
            (GuestCaller, Command::AsyncPendingImportCall(10)),
            (GuestCallee, Command::AsyncPendingExportComplete(10)),
            (GuestCaller, Command::AsyncPendingImportCall(11)),
            (GuestCallee, Command::AsyncPendingExportComplete(11)),
            (GuestCaller, Command::AsyncPendingImportCall(12)),
            (GuestCallee, Command::AsyncPendingExportComplete(12)),
            (GuestCaller, Command::AsyncPendingImportCall(13)),
            (GuestCallee, Command::AsyncPendingExportComplete(13)),
            (GuestCaller, Command::AsyncPendingImportCall(14)),
            (GuestCallee, Command::AsyncPendingExportComplete(14)),
            (GuestCaller, Command::AsyncPendingImportCall(15)),
            (GuestCallee, Command::AsyncPendingExportComplete(15)),
            (GuestCaller, Command::AsyncPendingImportCall(16)),
            (GuestCallee, Command::AsyncPendingExportComplete(16)),
            (GuestCaller, Command::AsyncPendingImportCall(17)),
            (GuestCallee, Command::AsyncPendingExportComplete(17)),
            (GuestCaller, Command::AsyncPendingImportCall(18)),
            (GuestCallee, Command::AsyncPendingExportComplete(18)),
            (GuestCaller, Command::AsyncPendingImportCall(19)),
            (GuestCallee, Command::AsyncPendingExportComplete(19)),
            (GuestCaller, Command::AsyncPendingImportCall(20)),
            (GuestCallee, Command::AsyncPendingExportComplete(20)),
            (GuestCaller, Command::AsyncPendingImportCall(21)),
            (GuestCallee, Command::AsyncPendingExportComplete(21)),
            (GuestCaller, Command::AsyncPendingImportCall(22)),
            (GuestCallee, Command::AsyncPendingExportComplete(22)),
            (GuestCaller, Command::AsyncPendingImportCall(23)),
            (GuestCallee, Command::AsyncPendingExportComplete(23)),
            (GuestCaller, Command::AsyncPendingImportCall(24)),
            (GuestCallee, Command::AsyncPendingExportComplete(24)),
            (GuestCaller, Command::AsyncPendingImportCall(25)),
            (GuestCallee, Command::AsyncPendingExportComplete(25)),
            (GuestCaller, Command::AsyncPendingImportCall(26)),
            (GuestCallee, Command::AsyncPendingExportComplete(26)),
            (GuestCaller, Command::AsyncPendingImportCall(27)),
            (GuestCallee, Command::AsyncPendingExportComplete(27)),
            (GuestCaller, Command::AsyncPendingImportCall(28)),
            (GuestCallee, Command::AsyncPendingExportComplete(28)),
            (GuestCaller, Command::AsyncPendingImportCall(29)),
            (GuestCallee, Command::AsyncPendingExportComplete(29)),
            (GuestCaller, Command::AsyncPendingImportCall(30)),
            (GuestCallee, Command::AsyncPendingExportComplete(30)),
            (GuestCaller, Command::AsyncPendingImportCall(31)),
            (GuestCallee, Command::AsyncPendingExportComplete(31)),
            (GuestCaller, Command::AsyncPendingImportCall(32)),
            (GuestCallee, Command::AsyncPendingExportComplete(32)),
            (GuestCaller, Command::AsyncPendingImportCall(33)),
            (GuestCallee, Command::AsyncPendingExportComplete(33)),
            (GuestCaller, Command::AsyncPendingImportCall(34)),
            (GuestCallee, Command::AsyncPendingExportComplete(34)),
            (GuestCaller, Command::AsyncPendingImportCall(35)),
            (GuestCallee, Command::AsyncPendingExportComplete(35)),
            (GuestCaller, Command::AsyncPendingImportCall(36)),
            (GuestCallee, Command::AsyncPendingExportComplete(36)),
            (GuestCaller, Command::AsyncPendingImportCall(37)),
            (GuestCallee, Command::AsyncPendingExportComplete(37)),
            (GuestCaller, Command::AsyncPendingImportAssertReady(36)),
        ];
        init();

        run(ComponentAsync {
            commands: COMMANDS.to_vec(),
        });
    }

    #[test]
    fn simple_stream1() {
        init();

        run(ComponentAsync {
            commands: vec![
                (HostCallee, Command::StreamNew(1)),
                (
                    HostCallee,
                    Command::StreamReadPending(StreamReadPayload {
                        stream: 1,
                        count: 2,
                    }),
                ),
                (HostCallee, Command::StreamCancelRead(1)),
                (GuestCaller, Command::SyncReadyCall),
                (
                    HostCallee,
                    Command::StreamWritePending(StreamWritePayload {
                        stream: 1,
                        item: 3,
                        count: 2,
                    }),
                ),
                (HostCallee, Command::StreamCancelWrite(1)),
                (HostCallee, Command::StreamDropWritable(1)),
                (
                    HostCallee,
                    Command::StreamReadDropped(StreamReadPayload {
                        stream: 1,
                        count: 1,
                    }),
                ),
            ],
        });
    }

    #[test]
    fn simple_stream3() {
        init();

        run(ComponentAsync {
            commands: vec![
                (GuestCaller, Command::StreamNew(26)),
                (
                    GuestCaller,
                    Command::StreamReadPending(StreamReadPayload {
                        stream: 26,
                        count: 10,
                    }),
                ),
                (GuestCaller, Command::StreamDropWritable(26)),
                (GuestCaller, Command::StreamReadAssertDropped(26)),
            ],
        });
    }

    #[test]
    fn simple_stream4() {
        init();

        run(ComponentAsync {
            commands: vec![
                (GuestCaller, Command::StreamNew(23)),
                (
                    GuestCaller,
                    Command::StreamWritePending(StreamWritePayload {
                        stream: 23,
                        item: 24,
                        count: 2,
                    }),
                ),
                (GuestCaller, Command::StreamGive(23)),
                (GuestCallee, Command::StreamDropReadable(23)),
                (
                    GuestCaller,
                    Command::StreamWriteAssertDropped(StreamReadPayload {
                        stream: 23,
                        count: 0,
                    }),
                ),
            ],
        });
    }

    #[test]
    fn zero_length_behavior() {
        init();

        run(ComponentAsync {
            commands: vec![
                (GuestCaller, Command::StreamNew(10)),
                (HostCaller, Command::StreamTake(10)),
                (
                    GuestCaller,
                    Command::StreamWritePending(StreamWritePayload {
                        stream: 10,
                        item: 13,
                        count: 5,
                    }),
                ),
                (
                    HostCaller,
                    Command::StreamReadReady(StreamReadyPayload {
                        stream: 10,
                        item: 0,
                        op_count: 0,
                        ready_count: 0,
                    }),
                ),
                (
                    HostCaller,
                    Command::StreamReadReady(StreamReadyPayload {
                        stream: 10,
                        item: 0,
                        op_count: 0,
                        ready_count: 0,
                    }),
                ),
            ],
        });
    }
}
