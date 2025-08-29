use {
    super::util::{config, make_component},
    anyhow::Result,
    component_async_tests::{
        Ctx, closed_streams,
        util::{OneshotConsumer, OneshotProducer, PipeConsumer, PipeProducer},
    },
    futures::{
        SinkExt, StreamExt,
        channel::{mpsc, oneshot},
        future,
    },
    std::sync::{Arc, Mutex},
    wasmtime::{
        Engine, Store,
        component::{FutureReader, Linker, ResourceTable, StreamReader},
    },
    wasmtime_wasi::WasiCtxBuilder,
};

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
    {
        let (mut input_tx, input_rx) = mpsc::channel(1);
        let (output_tx, mut output_rx) = mpsc::channel(1);
        StreamReader::new(instance, &mut store, PipeProducer::new(input_rx))
            .pipe(&mut store, PipeConsumer::new(output_tx));

        instance
            .run_concurrent(&mut store, async |_| {
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

    // Next, test futures host->host
    {
        let (input_tx, input_rx) = oneshot::channel();
        let (output_tx, output_rx) = oneshot::channel();
        FutureReader::new(instance, &mut store, OneshotProducer::new(input_rx))
            .pipe(&mut store, OneshotConsumer::new(output_tx));

        instance
            .run_concurrent(&mut store, async |_| {
                _ = input_tx.send(value);
                assert_eq!(value, output_rx.await?);
                anyhow::Ok(())
            })
            .await??;
    }

    // Next, test stream host->guest
    {
        let (mut tx, rx) = mpsc::channel(1);
        let rx = StreamReader::new(instance, &mut store, PipeProducer::new(rx));

        let closed_streams = closed_streams::bindings::ClosedStreams::new(&mut store, &instance)?;

        let values = values.clone();

        instance
            .run_concurrent(&mut store, async move |accessor| {
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
        let rx = FutureReader::new(instance, &mut store, OneshotProducer::new(rx));
        let (_, rx_ignored) = oneshot::channel();
        let rx_ignored = FutureReader::new(instance, &mut store, OneshotProducer::new(rx_ignored));

        let closed_streams = closed_streams::bindings::ClosedStreams::new(&mut store, &instance)?;

        instance
            .run_concurrent(&mut store, async move |accessor| {
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
