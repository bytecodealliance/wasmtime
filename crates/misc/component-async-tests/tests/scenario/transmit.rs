use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use super::util::{config, make_component, test_run, test_run_with_count};
use anyhow::{Result, anyhow};
use cancel::exports::local::local::cancel::Mode;
use component_async_tests::transmit::bindings::exports::local::local::transmit::Control;
use component_async_tests::util::{MpscConsumer, MpscProducer, OneshotConsumer, OneshotProducer};
use component_async_tests::{Ctx, sleep, transmit};
use futures::{
    FutureExt, SinkExt, StreamExt, TryStreamExt,
    channel::{mpsc, oneshot},
    stream::FuturesUnordered,
};
use wasmtime::component::{
    Accessor, Component, FutureReader, HasSelf, Instance, Linker, ResourceTable, StreamReader, Val,
};
use wasmtime::{AsContextMut, Engine, Store};
use wasmtime_wasi::WasiCtxBuilder;

#[tokio::test]
pub async fn async_poll_synchronous() -> Result<()> {
    test_run(&[test_programs_artifacts::ASYNC_POLL_SYNCHRONOUS_COMPONENT]).await
}

#[tokio::test]
pub async fn async_poll_stackless() -> Result<()> {
    test_run(&[test_programs_artifacts::ASYNC_POLL_STACKLESS_COMPONENT]).await
}

pub mod cancel {
    wasmtime::component::bindgen!({
        path: "wit",
        world: "cancel-host",
        exports: { default: async | store },
    });
}

// No-op function; we only test this by composing it in `async_cancel_caller`
#[allow(
    dead_code,
    reason = "here only to make the `assert_test_exists` macro happy"
)]
pub fn async_cancel_callee() {}

#[tokio::test]
pub async fn async_cancel_caller() -> Result<()> {
    test_cancel(Mode::Normal).await
}

#[tokio::test]
pub async fn async_cancel_caller_leak_task_after_cancel() -> Result<()> {
    test_cancel(Mode::LeakTaskAfterCancel).await
}

#[tokio::test]
pub async fn async_trap_cancel_guest_after_start_cancelled() -> Result<()> {
    test_cancel_trap(Mode::TrapCancelGuestAfterStartCancelled).await
}

#[tokio::test]
pub async fn async_trap_cancel_guest_after_return_cancelled() -> Result<()> {
    test_cancel_trap(Mode::TrapCancelGuestAfterReturnCancelled).await
}

#[tokio::test]
pub async fn async_trap_cancel_guest_after_return() -> Result<()> {
    test_cancel_trap(Mode::TrapCancelGuestAfterReturn).await
}

#[tokio::test]
pub async fn async_trap_cancel_host_after_return_cancelled() -> Result<()> {
    test_cancel_trap(Mode::TrapCancelHostAfterReturnCancelled).await
}

#[tokio::test]
pub async fn async_trap_cancel_host_after_return() -> Result<()> {
    test_cancel_trap(Mode::TrapCancelHostAfterReturn).await
}

fn cancel_delay() -> u64 {
    // Miri-based builds are much slower to run, so we delay longer in that case
    // to ensure that async calls which the test expects to return `BLOCKED`
    // actually do so.
    //
    // TODO: Make this test (more) deterministic so that such tuning is not
    // necessary.
    if cfg!(miri) { 1000 } else { 10 }
}

async fn test_cancel_trap(mode: Mode) -> Result<()> {
    let message = "`subtask.cancel` called after terminal status delivered";
    let trap = test_cancel(mode).await.unwrap_err();
    assert!(
        format!("{trap:?}").contains(message),
        "expected `{message}`; got `{trap:?}`",
    );
    Ok(())
}

async fn test_cancel(mode: Mode) -> Result<()> {
    let engine = Engine::new(&config())?;

    let component = make_component(
        &engine,
        &[
            test_programs_artifacts::ASYNC_CANCEL_CALLER_COMPONENT,
            test_programs_artifacts::ASYNC_CANCEL_CALLEE_COMPONENT,
        ],
    )
    .await?;

    let mut linker = Linker::new(&engine);

    wasmtime_wasi::p2::add_to_linker_async(&mut linker)?;
    sleep::local::local::sleep::add_to_linker::<_, Ctx>(&mut linker, |ctx| ctx)?;

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
    let cancel_host = cancel::CancelHost::new(&mut store, &instance)?;
    instance
        .run_concurrent(&mut store, async move |accessor| {
            cancel_host
                .local_local_cancel()
                .call_run(accessor, mode, cancel_delay())
                .await
        })
        .await??;

    Ok(())
}

#[tokio::test]
pub async fn async_intertask_communication() -> Result<()> {
    test_run_with_count(
        &[test_programs_artifacts::ASYNC_INTERTASK_COMMUNICATION_COMPONENT],
        2,
    )
    .await
}

#[tokio::test]
pub async fn async_transmit_caller() -> Result<()> {
    test_run(&[
        test_programs_artifacts::ASYNC_TRANSMIT_CALLER_COMPONENT,
        test_programs_artifacts::ASYNC_TRANSMIT_CALLEE_COMPONENT,
    ])
    .await
}

#[tokio::test]
pub async fn async_transmit_callee() -> Result<()> {
    test_transmit(test_programs_artifacts::ASYNC_TRANSMIT_CALLEE_COMPONENT).await
}

pub trait TransmitTest {
    type Instance: Send + Sync;
    type Params;
    type Result: Send + Sync + 'static;

    fn instantiate(
        store: impl AsContextMut<Data = Ctx>,
        component: &Component,
        linker: &Linker<Ctx>,
    ) -> impl Future<Output = Result<(Self::Instance, Instance)>>;

    fn call<'a>(
        accessor: &'a Accessor<Ctx, HasSelf<Ctx>>,
        instance: &'a Self::Instance,
        params: Self::Params,
    ) -> impl Future<Output = Result<Self::Result>> + Send + 'a;

    fn into_params(
        control: StreamReader<Control>,
        caller_stream: StreamReader<String>,
        caller_future1: FutureReader<String>,
        caller_future2: FutureReader<String>,
    ) -> Self::Params;

    fn from_result(
        store: impl AsContextMut<Data = Ctx>,
        instance: Instance,
        result: Self::Result,
    ) -> Result<(
        StreamReader<String>,
        FutureReader<String>,
        FutureReader<String>,
    )>;
}

struct StaticTransmitTest;

impl TransmitTest for StaticTransmitTest {
    type Instance = transmit::bindings::TransmitCallee;
    type Params = (
        StreamReader<Control>,
        StreamReader<String>,
        FutureReader<String>,
        FutureReader<String>,
    );
    type Result = (
        StreamReader<String>,
        FutureReader<String>,
        FutureReader<String>,
    );

    async fn instantiate(
        mut store: impl AsContextMut<Data = Ctx>,
        component: &Component,
        linker: &Linker<Ctx>,
    ) -> Result<(Self::Instance, Instance)> {
        let instance = linker.instantiate_async(&mut store, component).await?;
        let callee = transmit::bindings::TransmitCallee::new(store, &instance)?;
        Ok((callee, instance))
    }

    fn call<'a>(
        accessor: &'a Accessor<Ctx, HasSelf<Ctx>>,
        instance: &'a Self::Instance,
        params: Self::Params,
    ) -> impl Future<Output = Result<Self::Result>> + Send + 'a {
        instance
            .local_local_transmit()
            .call_exchange(accessor, params.0, params.1, params.2, params.3)
    }

    fn into_params(
        control: StreamReader<Control>,
        caller_stream: StreamReader<String>,
        caller_future1: FutureReader<String>,
        caller_future2: FutureReader<String>,
    ) -> Self::Params {
        (control, caller_stream, caller_future1, caller_future2)
    }

    fn from_result(
        _: impl AsContextMut<Data = Ctx>,
        _: Instance,
        result: Self::Result,
    ) -> Result<(
        StreamReader<String>,
        FutureReader<String>,
        FutureReader<String>,
    )> {
        Ok(result)
    }
}

struct DynamicTransmitTest;

impl TransmitTest for DynamicTransmitTest {
    type Instance = Instance;
    type Params = Vec<Val>;
    type Result = Val;

    async fn instantiate(
        store: impl AsContextMut<Data = Ctx>,
        component: &Component,
        linker: &Linker<Ctx>,
    ) -> Result<(Self::Instance, Instance)> {
        let instance = linker.instantiate_async(store, component).await?;
        Ok((instance, instance))
    }

    async fn call<'a>(
        accessor: &'a Accessor<Ctx, HasSelf<Ctx>>,
        instance: &'a Self::Instance,
        params: Self::Params,
    ) -> Result<Self::Result> {
        let exchange_function = accessor.with(|mut store| {
            let transmit_instance = instance
                .get_export_index(store.as_context_mut(), None, "local:local/transmit")
                .ok_or_else(|| anyhow!("can't find `local:local/transmit` in instance"))?;
            let exchange_function = instance
                .get_export_index(
                    store.as_context_mut(),
                    Some(&transmit_instance),
                    "[async]exchange",
                )
                .ok_or_else(|| anyhow!("can't find `exchange` in instance"))?;
            instance
                .get_func(store.as_context_mut(), exchange_function)
                .ok_or_else(|| anyhow!("can't find `exchange` in instance"))
        })?;

        let mut results = vec![Val::Bool(false)];
        exchange_function
            .call_concurrent(accessor, &params, &mut results)
            .await?;
        Ok(results.pop().unwrap())
    }

    fn into_params(
        control: StreamReader<Control>,
        caller_stream: StreamReader<String>,
        caller_future1: FutureReader<String>,
        caller_future2: FutureReader<String>,
    ) -> Self::Params {
        vec![
            control.into_val(),
            caller_stream.into_val(),
            caller_future1.into_val(),
            caller_future2.into_val(),
        ]
    }

    fn from_result(
        mut store: impl AsContextMut<Data = Ctx>,
        instance: Instance,
        result: Self::Result,
    ) -> Result<(
        StreamReader<String>,
        FutureReader<String>,
        FutureReader<String>,
    )> {
        let Val::Tuple(fields) = result else {
            unreachable!()
        };
        let stream = StreamReader::from_val(store.as_context_mut(), instance, &fields[0])?;
        let future1 = FutureReader::from_val(store.as_context_mut(), instance, &fields[1])?;
        let future2 = FutureReader::from_val(store.as_context_mut(), instance, &fields[2])?;
        Ok((stream, future1, future2))
    }
}

async fn test_transmit(component: &str) -> Result<()> {
    test_transmit_with::<StaticTransmitTest>(component).await?;
    test_transmit_with::<DynamicTransmitTest>(component).await
}

async fn test_transmit_with<Test: TransmitTest + 'static>(component: &str) -> Result<()> {
    let engine = Engine::new(&config())?;

    let make_store = || {
        Store::new(
            &engine,
            Ctx {
                wasi: WasiCtxBuilder::new().inherit_stdio().build(),
                table: ResourceTable::default(),
                continue_: false,
                wakers: Arc::new(Mutex::new(None)),
            },
        )
    };

    let component = make_component(&engine, &[component]).await?;

    let mut linker = Linker::new(&engine);

    wasmtime_wasi::p2::add_to_linker_async(&mut linker)?;

    let mut store = make_store();

    let (test, instance) = Test::instantiate(&mut store, &component, &linker).await?;

    enum Event<Test: TransmitTest> {
        Result(Test::Result),
        ControlWriteA(mpsc::Sender<Control>),
        ControlWriteB(mpsc::Sender<Control>),
        ControlWriteC(mpsc::Sender<Control>),
        ControlWriteD,
        WriteA,
        ReadC(mpsc::Receiver<String>, Option<String>),
        ReadD(mpsc::Receiver<String>, Option<String>),
        ReadNone(Option<String>),
    }

    let (mut control_tx, control_rx) = mpsc::channel(1);
    let control_rx = StreamReader::new(instance, &mut store, MpscProducer::new(control_rx));
    let (mut caller_stream_tx, caller_stream_rx) = mpsc::channel(1);
    let caller_stream_rx =
        StreamReader::new(instance, &mut store, MpscProducer::new(caller_stream_rx));
    let (caller_future1_tx, caller_future1_rx) = oneshot::channel();
    let caller_future1_rx = FutureReader::new(
        instance,
        &mut store,
        OneshotProducer::new(caller_future1_rx),
    );
    let (_, caller_future2_rx) = oneshot::channel();
    let caller_future2_rx = FutureReader::new(
        instance,
        &mut store,
        OneshotProducer::new(caller_future2_rx),
    );
    let (callee_future1_tx, callee_future1_rx) = oneshot::channel();
    let (callee_stream_tx, callee_stream_rx) = mpsc::channel(1);
    instance
        .run_concurrent(&mut store, async |accessor| {
            let mut caller_future1_tx = Some(caller_future1_tx);
            let mut callee_future1_tx = Some(callee_future1_tx);
            let mut callee_future1_rx = Some(callee_future1_rx);
            let mut callee_stream_tx = Some(callee_stream_tx);
            let mut callee_stream_rx = Some(callee_stream_rx);
            let mut complete = false;
            let mut futures = FuturesUnordered::<
                Pin<Box<dyn Future<Output = Result<Event<Test>>> + Send>>,
            >::new();

            futures.push(
                async move {
                    control_tx.send(Control::ReadStream("a".into())).await?;
                    Ok(Event::ControlWriteA(control_tx))
                }
                .boxed(),
            );

            futures.push(
                async move {
                    caller_stream_tx.send(String::from("a")).await?;
                    Ok(Event::WriteA)
                }
                .boxed(),
            );

            futures.push(
                Test::call(
                    accessor,
                    &test,
                    Test::into_params(
                        control_rx,
                        caller_stream_rx,
                        caller_future1_rx,
                        caller_future2_rx,
                    ),
                )
                .map(|v| v.map(Event::Result))
                .boxed(),
            );

            while let Some(event) = futures.try_next().await? {
                match event {
                    Event::Result(result) => {
                        accessor.with(|mut store| {
                            let (callee_stream_rx, callee_future1_rx, _) =
                                Test::from_result(&mut store, instance, result)?;
                            callee_stream_rx.pipe(
                                &mut store,
                                MpscConsumer::new(callee_stream_tx.take().unwrap()),
                            );
                            callee_future1_rx.pipe(
                                &mut store,
                                OneshotConsumer::new(callee_future1_tx.take().unwrap()),
                            );
                            anyhow::Ok(())
                        })?;
                    }
                    Event::ControlWriteA(mut control_tx) => {
                        futures.push(
                            async move {
                                control_tx.send(Control::ReadFuture("b".into())).await?;
                                Ok(Event::ControlWriteB(control_tx))
                            }
                            .boxed(),
                        );
                    }
                    Event::WriteA => {
                        _ = caller_future1_tx.take().unwrap().send("b".into());
                        let mut callee_stream_rx = callee_stream_rx.take().unwrap();
                        futures.push(
                            async move {
                                let value = callee_stream_rx.next().await;
                                Ok(Event::ReadC(callee_stream_rx, value))
                            }
                            .boxed(),
                        );
                    }
                    Event::ControlWriteB(mut control_tx) => {
                        futures.push(
                            async move {
                                control_tx.send(Control::WriteStream("c".into())).await?;
                                Ok(Event::ControlWriteC(control_tx))
                            }
                            .boxed(),
                        );
                    }
                    Event::ControlWriteC(mut control_tx) => {
                        futures.push(
                            async move {
                                control_tx.send(Control::WriteFuture("d".into())).await?;
                                Ok(Event::ControlWriteD)
                            }
                            .boxed(),
                        );
                    }
                    Event::ReadC(callee_stream_rx, mut value) => {
                        assert_eq!(value.take().as_deref(), Some("c"));
                        futures.push(
                            callee_future1_rx
                                .take()
                                .unwrap()
                                .map(|v| Event::ReadD(callee_stream_rx, v.ok()))
                                .map(Ok)
                                .boxed(),
                        );
                    }
                    Event::ControlWriteD => {}
                    Event::ReadD(_, None) => unreachable!(),
                    Event::ReadD(mut callee_stream_rx, Some(value)) => {
                        assert_eq!(&value, "d");
                        futures.push(
                            async move { Ok(Event::ReadNone(callee_stream_rx.next().await)) }
                                .boxed(),
                        );
                    }
                    Event::ReadNone(Some(_)) => unreachable!(),
                    Event::ReadNone(None) => {
                        complete = true;
                    }
                }
            }

            assert!(complete);

            anyhow::Ok(())
        })
        .await??;
    Ok(())
}
