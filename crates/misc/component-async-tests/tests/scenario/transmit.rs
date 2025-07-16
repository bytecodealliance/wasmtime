use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use super::util::{config, make_component, test_run, test_run_with_count};
use anyhow::{Result, anyhow};
use cancel::exports::local::local::cancel::Mode;
use component_async_tests::transmit::bindings::exports::local::local::transmit::Control;
use component_async_tests::{Ctx, sleep, transmit};
use futures::{
    future::{self, FutureExt},
    stream::{FuturesUnordered, TryStreamExt},
};
use wasmtime::component::{
    Accessor, Component, HasSelf, HostFuture, HostStream, Instance, Linker, ResourceTable,
    StreamReader, StreamWriter, Val,
};
use wasmtime::{AsContextMut, Engine, Store};
use wasmtime_wasi::p2::WasiCtxBuilder;

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
        concurrent_imports: true,
        concurrent_exports: true,
        async: {
            only_imports: [],
        }
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
        .run_with(&mut store, async move |accessor| {
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
        control: HostStream<Control>,
        caller_stream: HostStream<String>,
        caller_future1: HostFuture<String>,
        caller_future2: HostFuture<String>,
    ) -> Self::Params;

    fn from_result(
        store: impl AsContextMut<Data = Ctx>,
        instance: Instance,
        result: Self::Result,
    ) -> Result<(HostStream<String>, HostFuture<String>, HostFuture<String>)>;
}

struct StaticTransmitTest;

impl TransmitTest for StaticTransmitTest {
    type Instance = transmit::bindings::TransmitCallee;
    type Params = (
        HostStream<Control>,
        HostStream<String>,
        HostFuture<String>,
        HostFuture<String>,
    );
    type Result = (HostStream<String>, HostFuture<String>, HostFuture<String>);

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
        control: HostStream<Control>,
        caller_stream: HostStream<String>,
        caller_future1: HostFuture<String>,
        caller_future2: HostFuture<String>,
    ) -> Self::Params {
        (control, caller_stream, caller_future1, caller_future2)
    }

    fn from_result(
        _: impl AsContextMut<Data = Ctx>,
        _: Instance,
        result: Self::Result,
    ) -> Result<(HostStream<String>, HostFuture<String>, HostFuture<String>)> {
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

    fn call<'a>(
        accessor: &'a Accessor<Ctx, HasSelf<Ctx>>,
        instance: &'a Self::Instance,
        params: Self::Params,
    ) -> impl Future<Output = Result<Self::Result>> + Send + 'a {
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
        });

        match exchange_function {
            Ok(exchange_function) => exchange_function
                .call_concurrent(accessor, params)
                .map(|v| v.map(|v| v.into_iter().next().unwrap()))
                .boxed(),
            Err(e) => future::ready(Err(e)).boxed(),
        }
    }

    fn into_params(
        control: HostStream<Control>,
        caller_stream: HostStream<String>,
        caller_future1: HostFuture<String>,
        caller_future2: HostFuture<String>,
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
    ) -> Result<(HostStream<String>, HostFuture<String>, HostFuture<String>)> {
        let Val::Tuple(fields) = result else {
            unreachable!()
        };
        let stream = HostStream::from_val(store.as_context_mut(), instance, &fields[0])?;
        let future1 = HostFuture::from_val(store.as_context_mut(), instance, &fields[1])?;
        let future2 = HostFuture::from_val(store.as_context_mut(), instance, &fields[2])?;
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
        ControlWriteA(Option<StreamWriter<Option<Control>>>),
        ControlWriteB(Option<StreamWriter<Option<Control>>>),
        ControlWriteC(Option<StreamWriter<Option<Control>>>),
        ControlWriteD,
        WriteA,
        WriteB(bool),
        ReadC(Option<StreamReader<Option<String>>>, Option<String>),
        ReadD(Option<String>),
        ReadNone(Option<StreamReader<Option<String>>>),
    }

    let (control_tx, control_rx) = instance.stream::<_, _, Option<_>>(&mut store)?;
    let (caller_stream_tx, caller_stream_rx) = instance.stream::<_, _, Option<_>>(&mut store)?;
    let (caller_future1_tx, caller_future1_rx) = instance.future(|| unreachable!(), &mut store)?;
    let (_caller_future2_tx, caller_future2_rx) = instance.future(|| unreachable!(), &mut store)?;

    instance
        .run_with(&mut store, async move |accessor| {
            let mut futures = FuturesUnordered::<
                Pin<Box<dyn Future<Output = Result<Event<Test>>> + Send>>,
            >::new();
            let mut caller_future1_tx = Some(caller_future1_tx);
            let mut callee_stream_rx = None;
            let mut callee_future1_rx = None;
            let mut complete = false;

            futures.push(
                control_tx
                    .write_all(accessor, Some(Control::ReadStream("a".into())))
                    .map(|(w, _)| Ok(Event::ControlWriteA(w)))
                    .boxed(),
            );

            futures.push(
                caller_stream_tx
                    .write_all(accessor, Some(String::from("a")))
                    .map(|_| Ok(Event::WriteA))
                    .boxed(),
            );

            futures.push(
                Test::call(
                    accessor,
                    &test,
                    Test::into_params(
                        control_rx.into(),
                        caller_stream_rx.into(),
                        caller_future1_rx.into(),
                        caller_future2_rx.into(),
                    ),
                )
                .map(|v| v.map(Event::Result))
                .boxed(),
            );

            while let Some(event) = futures.try_next().await? {
                match event {
                    Event::Result(result) => {
                        accessor.with(|mut store| {
                            let results = Test::from_result(&mut store, instance, result)?;
                            callee_stream_rx = Some(results.0.into_reader(&mut store));
                            callee_future1_rx = Some(results.1.into_reader(&mut store));
                            anyhow::Ok(())
                        })?;
                    }
                    Event::ControlWriteA(tx) => {
                        futures.push(
                            tx.unwrap()
                                .write_all(accessor, Some(Control::ReadFuture("b".into())))
                                .map(|(w, _)| Ok(Event::ControlWriteB(w)))
                                .boxed(),
                        );
                    }
                    Event::WriteA => {
                        futures.push(
                            caller_future1_tx
                                .take()
                                .unwrap()
                                .write(accessor, "b".into())
                                .map(Event::WriteB)
                                .map(Ok)
                                .boxed(),
                        );
                    }
                    Event::ControlWriteB(tx) => {
                        futures.push(
                            tx.unwrap()
                                .write_all(accessor, Some(Control::WriteStream("c".into())))
                                .map(|(w, _)| Ok(Event::ControlWriteC(w)))
                                .boxed(),
                        );
                    }
                    Event::WriteB(delivered) => {
                        assert!(delivered);
                        futures.push(
                            callee_stream_rx
                                .take()
                                .unwrap()
                                .read(accessor, None)
                                .map(|(r, b)| Ok(Event::ReadC(r, b)))
                                .boxed(),
                        );
                    }
                    Event::ControlWriteC(tx) => {
                        futures.push(
                            tx.unwrap()
                                .write_all(accessor, Some(Control::WriteFuture("d".into())))
                                .map(|_| Ok(Event::ControlWriteD))
                                .boxed(),
                        );
                    }
                    Event::ReadC(None, _) => unreachable!(),
                    Event::ReadC(Some(rx), mut value) => {
                        assert_eq!(value.take().as_deref(), Some("c"));
                        futures.push(
                            callee_future1_rx
                                .take()
                                .unwrap()
                                .read(accessor)
                                .map(Event::ReadD)
                                .map(Ok)
                                .boxed(),
                        );
                        callee_stream_rx = Some(rx);
                    }
                    Event::ControlWriteD => {}
                    Event::ReadD(None) => unreachable!(),
                    Event::ReadD(Some(value)) => {
                        assert_eq!(&value, "d");
                        futures.push(
                            callee_stream_rx
                                .take()
                                .unwrap()
                                .read(accessor, None)
                                .map(|(r, _)| Ok(Event::ReadNone(r)))
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
        .await?
}
