use {
    super::util::{config, make_component},
    anyhow::Result,
    component_async_tests::{Ctx, closed_streams},
    futures::{
        future::FutureExt,
        stream::{FuturesUnordered, StreamExt, TryStreamExt},
    },
    std::{
        future::Future,
        pin::pin,
        sync::{Arc, Mutex},
        task::{Context, Waker},
    },
    wasmtime::{
        Engine, Store,
        component::{Linker, ResourceTable, StreamReader, StreamWriter, VecBuffer, WithAccessor},
    },
    wasmtime_wasi::p2::WasiCtxBuilder,
};

#[tokio::test]
pub async fn async_watch_streams() -> Result<()> {
    use wasmtime::component::{DropWithStore, DropWithStoreAndValue};

    let engine = Engine::new(&config())?;

    let mut store = Store::new(
        &engine,
        Ctx {
            wasi: WasiCtxBuilder::new().inherit_stdio().build(),
            table: ResourceTable::default(),
            continue_: false,
            wakers: Arc::new(Mutex::new(None)),
        },
    );

    let mut linker = Linker::new(&engine);

    wasmtime_wasi::p2::add_to_linker_async(&mut linker)?;

    let component = make_component(
        &engine,
        &[test_programs_artifacts::ASYNC_CLOSED_STREAMS_COMPONENT],
    )
    .await?;

    let instance = linker.instantiate_async(&mut store, &component).await?;

    // Test watching and then dropping the read end of a stream.
    let (mut tx, rx) = instance.stream::<u8>(&mut store)?;
    instance
        .run_concurrent(&mut store, async |store| {
            futures::join!(tx.watch_reader(store), async { rx.drop_with(store) }).1
        })
        .await??;

    // Test dropping and then watching the read end of a stream.
    let (mut tx, rx) = instance.stream::<u8>(&mut store)?;
    instance
        .run_concurrent(&mut store, async |store| {
            rx.drop_with(store)?;
            tx.watch_reader(store).await;
            anyhow::Ok(())
        })
        .await??;

    // Test watching and then dropping the write end of a stream.
    let (tx, mut rx) = instance.stream::<u8>(&mut store)?;
    instance
        .run_concurrent(&mut store, async |store| {
            futures::join!(rx.watch_writer(store), async { tx.drop_with(store) }).1
        })
        .await??;

    // Test dropping and then watching the write end of a stream.
    let (tx, mut rx) = instance.stream::<u8>(&mut store)?;
    instance
        .run_concurrent(&mut store, async |store| {
            tx.drop_with(store)?;
            rx.watch_writer(store).await;
            anyhow::Ok(())
        })
        .await??;

    // Test watching and then dropping the read end of a future.
    let (mut tx, rx) = instance.future::<u8>(&mut store)?;
    instance
        .run_concurrent(&mut store, async |store| {
            futures::join!(tx.watch_reader(store), async { rx.drop_with(store) }).1
        })
        .await??;

    // Test dropping and then watching the read end of a future.
    let (mut tx, rx) = instance.future::<u8>(&mut store)?;
    instance
        .run_concurrent(&mut store, async |store| {
            rx.drop_with(store)?;
            tx.watch_reader(store).await;
            anyhow::Ok(())
        })
        .await??;

    // Test watching and then dropping the write end of a future.
    let (tx, mut rx) = instance.future::<u8>(&mut store)?;
    instance
        .run_concurrent(&mut store, async |store| {
            futures::join!(rx.watch_writer(store), async { tx.drop_with(store, 42) }).1
        })
        .await??;

    // Test dropping and then watching the write end of a future.
    let (tx, mut rx) = instance.future::<u8>(&mut store)?;
    instance
        .run_concurrent(&mut store, async |store| {
            tx.drop_with(store, 42)?;
            rx.watch_writer(store).await;
            anyhow::Ok(())
        })
        .await??;

    enum Event<'a> {
        Write(Option<WithAccessor<'a, StreamWriter<u8>, Ctx>>),
        Read(Option<WithAccessor<'a, StreamReader<u8>, Ctx>>, Option<u8>),
    }

    // Test watching, then writing to, then dropping, then writing again to the
    // read end of a stream.
    let (tx, rx) = instance.stream(&mut store)?;
    instance
        .run_concurrent(&mut store, async move |store| -> wasmtime::Result<_> {
            let mut tx = WithAccessor::new(store, tx);
            let mut rx = WithAccessor::new(store, rx);
            let mut futures = FuturesUnordered::new();
            assert!(
                pin!(tx.watch_reader(store))
                    .poll(&mut Context::from_waker(&Waker::noop()))
                    .is_pending()
            );
            futures.push(
                async move {
                    tx.write_all(store, Some(42)).await;
                    let w = if tx.is_closed() { None } else { Some(tx) };
                    anyhow::Ok(Event::Write(w))
                }
                .boxed(),
            );
            futures.push(
                async move {
                    let b = rx.read(store, None).await;
                    let r = if rx.is_closed() { None } else { Some(rx) };
                    Ok(Event::Read(r, b))
                }
                .boxed(),
            );
            let mut rx = None;
            let mut tx = None;
            while let Some(event) = futures.try_next().await? {
                match event {
                    Event::Write(None) => unreachable!(),
                    Event::Write(Some(new_tx)) => tx = Some(new_tx),
                    Event::Read(None, _) => unreachable!(),
                    Event::Read(Some(new_rx), mut buffer) => {
                        assert_eq!(buffer.take(), Some(42));
                        rx = Some(new_rx);
                    }
                }
            }
            drop(rx);

            let mut tx = tx.take().unwrap();
            tx.watch_reader(store).await;
            tx.write_all(store, Some(42)).await;
            assert!(tx.is_closed());
            Ok(())
        })
        .await??;

    Ok(())
}

#[tokio::test]
pub async fn async_closed_streams() -> Result<()> {
    test_closed_streams(false).await
}

#[tokio::test]
pub async fn async_closed_streams_with_watch() -> Result<()> {
    test_closed_streams(true).await
}

pub async fn test_closed_streams(watch: bool) -> Result<()> {
    let engine = Engine::new(&config())?;

    let mut store = Store::new(
        &engine,
        Ctx {
            wasi: WasiCtxBuilder::new().inherit_stdio().build(),
            table: ResourceTable::default(),
            continue_: false,
            wakers: Arc::new(Mutex::new(None)),
        },
    );

    let mut linker = Linker::new(&engine);

    wasmtime_wasi::p2::add_to_linker_async(&mut linker)?;

    let component = make_component(
        &engine,
        &[test_programs_artifacts::ASYNC_CLOSED_STREAMS_COMPONENT],
    )
    .await?;

    let instance = linker.instantiate_async(&mut store, &component).await?;

    enum StreamEvent<'a> {
        FirstWrite(Option<WithAccessor<'a, StreamWriter<u8>, Ctx>>),
        FirstRead(Option<WithAccessor<'a, StreamReader<u8>, Ctx>>, Vec<u8>),
        SecondWrite(Option<WithAccessor<'a, StreamWriter<u8>, Ctx>>),
        GuestCompleted,
    }

    enum FutureEvent {
        Write(bool),
        Read(Option<u8>),
        WriteIgnored(bool),
        GuestCompleted,
    }

    let values = vec![42_u8, 43, 44];

    let value = 42_u8;

    // First, test stream host->host
    {
        let (tx, rx) = instance.stream(&mut store)?;
        let values = values.clone();

        instance
            .run_concurrent(&mut store, async move |store| -> wasmtime::Result<_> {
                let mut tx = WithAccessor::new(store, tx);
                let mut rx = WithAccessor::new(store, rx);

                let mut futures = FuturesUnordered::new();
                futures.push({
                    let values = values.clone();
                    async move {
                        tx.write_all(store, VecBuffer::from(values)).await;
                        anyhow::Ok(StreamEvent::FirstWrite(if tx.is_closed() {
                            None
                        } else {
                            Some(tx)
                        }))
                    }
                    .boxed()
                });
                futures.push(
                    async move {
                        let b = rx.read(store, Vec::with_capacity(3)).await;
                        let r = if rx.is_closed() { None } else { Some(rx) };
                        Ok(StreamEvent::FirstRead(r, b))
                    }
                    .boxed(),
                );

                let mut count = 0;
                while let Some(event) = futures.try_next().await? {
                    count += 1;
                    match event {
                        StreamEvent::FirstWrite(Some(mut tx)) => {
                            if watch {
                                futures.push(
                                    async move {
                                        tx.watch_reader(store).await;
                                        Ok(StreamEvent::SecondWrite(None))
                                    }
                                    .boxed(),
                                );
                            } else {
                                futures.push({
                                    let values = values.clone();
                                    async move {
                                        tx.write_all(store, VecBuffer::from(values)).await;
                                        Ok(StreamEvent::SecondWrite(if tx.is_closed() {
                                            None
                                        } else {
                                            Some(tx)
                                        }))
                                    }
                                    .boxed()
                                });
                            }
                        }
                        StreamEvent::FirstWrite(None) => {
                            panic!("first write should have been accepted")
                        }
                        StreamEvent::FirstRead(Some(_), results) => {
                            assert_eq!(values, results);
                        }
                        StreamEvent::FirstRead(None, _) => unreachable!(),
                        StreamEvent::SecondWrite(None) => {}
                        StreamEvent::SecondWrite(Some(_)) => {
                            panic!("second write should _not_ have been accepted")
                        }
                        StreamEvent::GuestCompleted => unreachable!(),
                    }
                }

                assert_eq!(count, 3);
                Ok(())
            })
            .await??;
    }

    // Next, test futures host->host
    {
        let (tx, rx) = instance.future(&mut store)?;
        let (mut tx_ignored, rx_ignored) = instance.future(&mut store)?;

        instance
            .run_concurrent(&mut store, async move |store| {
                let rx_ignored = WithAccessor::new(store, rx_ignored);

                let mut futures = FuturesUnordered::new();
                futures.push(tx.write(store, value).map(FutureEvent::Write).boxed());
                futures.push(rx.read(store).map(FutureEvent::Read).boxed());
                if watch {
                    futures.push(
                        tx_ignored
                            .watch_reader(store)
                            .map(|()| FutureEvent::WriteIgnored(false))
                            .boxed(),
                    );
                } else {
                    futures.push(
                        tx_ignored
                            .write(store, value)
                            .map(FutureEvent::WriteIgnored)
                            .boxed(),
                    );
                }
                drop(rx_ignored);

                let mut count = 0;
                while let Some(event) = futures.next().await {
                    count += 1;
                    match event {
                        FutureEvent::Write(delivered) => {
                            assert!(delivered);
                        }
                        FutureEvent::Read(Some(result)) => {
                            assert_eq!(value, result);
                        }
                        FutureEvent::Read(None) => panic!("read should have succeeded"),
                        FutureEvent::WriteIgnored(delivered) => {
                            assert!(!delivered);
                        }
                        FutureEvent::GuestCompleted => unreachable!(),
                    }
                }

                assert_eq!(count, 3);
                anyhow::Ok(())
            })
            .await??;
    }

    // Next, test stream host->guest
    {
        let (tx, rx) = instance.stream(&mut store)?;

        let closed_streams = closed_streams::bindings::ClosedStreams::new(&mut store, &instance)?;

        let values = values.clone();

        instance
            .run_concurrent(&mut store, async move |accessor| {
                let mut tx = WithAccessor::new(accessor, tx);

                let mut futures = FuturesUnordered::new();
                futures.push(
                    closed_streams
                        .local_local_closed()
                        .call_read_stream(accessor, rx, values.clone())
                        .map(|v| v.map(|()| StreamEvent::GuestCompleted))
                        .boxed(),
                );
                futures.push({
                    let values = values.clone();
                    async move {
                        tx.write_all(accessor, VecBuffer::from(values)).await;
                        let w = if tx.is_closed() { None } else { Some(tx) };
                        Ok(StreamEvent::FirstWrite(w))
                    }
                    .boxed()
                });

                let mut count = 0;
                while let Some(event) = futures.try_next().await? {
                    count += 1;
                    match event {
                        StreamEvent::FirstWrite(Some(mut tx)) => {
                            if watch {
                                futures.push(
                                    async move {
                                        tx.watch_reader(accessor).await;
                                        Ok(StreamEvent::SecondWrite(None))
                                    }
                                    .boxed(),
                                );
                            } else {
                                futures.push({
                                    let values = values.clone();
                                    async move {
                                        tx.write_all(accessor, VecBuffer::from(values)).await;
                                        let w = if tx.is_closed() { None } else { Some(tx) };
                                        Ok(StreamEvent::SecondWrite(w))
                                    }
                                    .boxed()
                                });
                            }
                        }
                        StreamEvent::FirstWrite(None) => {
                            panic!("first write should have been accepted")
                        }
                        StreamEvent::FirstRead(_, _) => unreachable!(),
                        StreamEvent::SecondWrite(None) => {}
                        StreamEvent::SecondWrite(Some(_)) => {
                            panic!("second write should _not_ have been accepted")
                        }
                        StreamEvent::GuestCompleted => {}
                    }
                }

                assert_eq!(count, 3);

                anyhow::Ok(())
            })
            .await??;
    }

    // Next, test futures host->guest
    {
        let (tx, rx) = instance.future(&mut store)?;
        let (mut tx_ignored, rx_ignored) = instance.future(&mut store)?;

        let closed_streams = closed_streams::bindings::ClosedStreams::new(&mut store, &instance)?;

        instance
            .run_concurrent(&mut store, async move |accessor| {
                let mut futures = FuturesUnordered::new();
                futures.push(
                    closed_streams
                        .local_local_closed()
                        .call_read_future(accessor, rx, value, rx_ignored)
                        .map(|v| v.map(|()| FutureEvent::GuestCompleted))
                        .boxed(),
                );
                futures.push(
                    tx.write(accessor, value)
                        .map(FutureEvent::Write)
                        .map(Ok)
                        .boxed(),
                );
                if watch {
                    futures.push(
                        tx_ignored
                            .watch_reader(accessor)
                            .map(|()| Ok(FutureEvent::WriteIgnored(false)))
                            .boxed(),
                    );
                } else {
                    futures.push(
                        tx_ignored
                            .write(accessor, value)
                            .map(FutureEvent::WriteIgnored)
                            .map(Ok)
                            .boxed(),
                    );
                }

                let mut count = 0;
                while let Some(event) = futures.try_next().await? {
                    count += 1;
                    match event {
                        FutureEvent::Write(delivered) => {
                            assert!(delivered);
                        }
                        FutureEvent::Read(_) => unreachable!(),
                        FutureEvent::WriteIgnored(delivered) => {
                            assert!(!delivered);
                        }
                        FutureEvent::GuestCompleted => {}
                    }
                }

                assert_eq!(count, 3);

                anyhow::Ok(())
            })
            .await??;
    }

    Ok(())
}
