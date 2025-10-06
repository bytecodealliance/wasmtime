use {
    super::util::{config, make_component},
    anyhow::Result,
    component_async_tests::{
        Ctx, closed_streams,
        util::{OneshotConsumer, OneshotProducer, PipeConsumer, PipeProducer},
    },
    futures::{
        Sink, SinkExt, Stream, StreamExt,
        channel::{mpsc, oneshot},
        future,
    },
    std::{
        pin::Pin,
        sync::{Arc, Mutex},
        task::{Context, Poll},
    },
    wasmtime::{
        Engine, Store, StoreContextMut,
        component::{
            Destination, FutureReader, Linker, ResourceTable, Source, StreamConsumer,
            StreamProducer, StreamReader, StreamResult,
        },
    },
    wasmtime_wasi::WasiCtxBuilder,
};

pub struct DirectPipeProducer<S>(S);

impl<D, S: Stream<Item = u8> + Send + 'static> StreamProducer<D> for DirectPipeProducer<S> {
    type Item = u8;
    type Buffer = Option<u8>;

    fn poll_produce<'a>(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        store: StoreContextMut<D>,
        destination: Destination<'a, Self::Item, Self::Buffer>,
        finish: bool,
    ) -> Poll<Result<StreamResult>> {
        // SAFETY: This is a standard pin-projection, and we never move
        // out of `self`.
        let stream = unsafe { self.map_unchecked_mut(|v| &mut v.0) };

        match stream.poll_next(cx) {
            Poll::Pending => {
                if finish {
                    Poll::Ready(Ok(StreamResult::Cancelled))
                } else {
                    Poll::Pending
                }
            }
            Poll::Ready(Some(item)) => {
                let mut destination = destination.as_direct(store, 1);
                destination.remaining()[0] = item;
                destination.mark_written(1);
                Poll::Ready(Ok(StreamResult::Completed))
            }
            Poll::Ready(None) => Poll::Ready(Ok(StreamResult::Dropped)),
        }
    }
}

pub struct DirectPipeConsumer<S>(S);

impl<D, S: Sink<u8, Error: std::error::Error + Send + Sync> + Send + 'static> StreamConsumer<D>
    for DirectPipeConsumer<S>
{
    type Item = u8;

    fn poll_consume(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        store: StoreContextMut<D>,
        source: Source<Self::Item>,
        finish: bool,
    ) -> Poll<Result<StreamResult>> {
        // SAFETY: This is a standard pin-projection, and we never move
        // out of `self`.
        let mut sink = unsafe { self.map_unchecked_mut(|v| &mut v.0) };

        let on_pending = || {
            if finish {
                Poll::Ready(Ok(StreamResult::Cancelled))
            } else {
                Poll::Pending
            }
        };

        match sink.as_mut().poll_flush(cx) {
            Poll::Pending => on_pending(),
            Poll::Ready(result) => {
                result?;
                match sink.as_mut().poll_ready(cx) {
                    Poll::Pending => on_pending(),
                    Poll::Ready(result) => {
                        result?;
                        let mut source = source.as_direct(store);
                        let item = source.remaining()[0];
                        source.mark_read(1);
                        sink.start_send(item)?;
                        Poll::Ready(Ok(StreamResult::Completed))
                    }
                }
            }
        }
    }
}

#[tokio::test]
pub async fn async_closed_streams() -> Result<()> {
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

    let values = vec![42_u8, 43, 44];

    let value = 42_u8;

    // First, test stream host->host
    for direct_producer in [true, false] {
        for direct_consumer in [true, false] {
            let (mut input_tx, input_rx) = mpsc::channel(1);
            let (output_tx, mut output_rx) = mpsc::channel(1);
            let reader = if direct_producer {
                StreamReader::new(&mut store, DirectPipeProducer(input_rx))
            } else {
                StreamReader::new(&mut store, PipeProducer::new(input_rx))
            };
            if direct_consumer {
                reader.pipe(&mut store, DirectPipeConsumer(output_tx));
            } else {
                reader.pipe(&mut store, PipeConsumer::new(output_tx));
            }

            store
                .run_concurrent(async |_| {
                    let (a, b) = future::join(
                        async {
                            for &value in &values {
                                input_tx.send(value).await?;
                            }
                            drop(input_tx);
                            anyhow::Ok(())
                        },
                        async {
                            for &value in &values {
                                assert_eq!(Some(value), output_rx.next().await);
                            }
                            assert!(output_rx.next().await.is_none());
                            Ok(())
                        },
                    )
                    .await;

                    a.and(b)
                })
                .await??;
        }
    }

    // Next, test futures host->host
    {
        let (input_tx, input_rx) = oneshot::channel();
        let (output_tx, output_rx) = oneshot::channel();
        FutureReader::new(&mut store, OneshotProducer::new(input_rx))
            .pipe(&mut store, OneshotConsumer::new(output_tx));

        store
            .run_concurrent(async |_| {
                _ = input_tx.send(value);
                assert_eq!(value, output_rx.await?);
                anyhow::Ok(())
            })
            .await??;
    }

    // Next, test stream host->guest
    {
        let (mut tx, rx) = mpsc::channel(1);
        let rx = StreamReader::new(&mut store, PipeProducer::new(rx));

        let closed_streams = closed_streams::bindings::ClosedStreams::new(&mut store, &instance)?;

        let values = values.clone();

        store
            .run_concurrent(async move |accessor| {
                let (a, b) = future::join(
                    async {
                        for &value in &values {
                            tx.send(value).await?;
                        }
                        drop(tx);
                        Ok(())
                    },
                    closed_streams.local_local_closed().call_read_stream(
                        accessor,
                        rx,
                        values.clone(),
                    ),
                )
                .await;

                a.and(b)
            })
            .await??;
    }

    // Next, test futures host->guest
    {
        let (tx, rx) = oneshot::channel();
        let rx = FutureReader::new(&mut store, OneshotProducer::new(rx));
        let (_, rx_ignored) = oneshot::channel();
        let rx_ignored = FutureReader::new(&mut store, OneshotProducer::new(rx_ignored));

        let closed_streams = closed_streams::bindings::ClosedStreams::new(&mut store, &instance)?;

        store
            .run_concurrent(async move |accessor| {
                _ = tx.send(value);
                closed_streams
                    .local_local_closed()
                    .call_read_future(accessor, rx, value, rx_ignored)
                    .await
            })
            .await??;
    }

    Ok(())
}

mod closed_stream {
    wasmtime::component::bindgen!({
        path: "wit",
        world: "closed-stream-guest",
        exports: { default: store | async },
    });
}

#[tokio::test]
pub async fn async_closed_stream() -> Result<()> {
    let engine = Engine::new(&config())?;

    let component = make_component(
        &engine,
        &[test_programs_artifacts::ASYNC_CLOSED_STREAM_COMPONENT],
    )
    .await?;

    let mut linker = Linker::new(&engine);

    wasmtime_wasi::p2::add_to_linker_async(&mut linker)?;

    let mut store = Store::new(
        &engine,
        Ctx {
            wasi: WasiCtxBuilder::new().inherit_stdio().build(),
            table: ResourceTable::default(),
            continue_: false,
            wakers: Arc::new(Mutex::new(None)),
        },
    );

    let instance = linker.instantiate_async(&mut store, &component).await?;
    let guest = closed_stream::ClosedStreamGuest::new(&mut store, &instance)?;
    store
        .run_concurrent(async move |accessor| {
            let stream = guest.local_local_closed_stream().call_get(accessor).await?;

            let (tx, mut rx) = mpsc::channel(1);
            accessor.with(move |store| stream.pipe(store, PipeConsumer::new(tx)));
            assert!(rx.next().await.is_none());

            Ok(())
        })
        .await?
}
