use std::future::Future;
use std::pin::Pin;
use std::task::{self, Context, Poll};

use super::util::{config, make_component, test_run, test_run_with_count};
use cancel::exports::local::local::cancel::Mode;
use component_async_tests::transmit::bindings::exports::local::local::transmit::Control;
use component_async_tests::util::{OneshotConsumer, OneshotProducer, PipeConsumer, PipeProducer};
use component_async_tests::{Ctx, transmit, yield_};
use futures::{
    FutureExt, SinkExt, StreamExt, TryStreamExt,
    channel::{mpsc, oneshot},
    stream::FuturesUnordered,
};
use wasmtime::component::{
    Accessor, Component, Destination, FutureConsumer, FutureProducer, FutureReader, HasSelf,
    Instance, Linker, ResourceTable, Source, StreamConsumer, StreamProducer, StreamReader,
    StreamResult, Val,
};
use wasmtime::{AsContextMut, Engine, Result, Store, StoreContextMut, format_err};
use wasmtime_wasi::WasiCtxBuilder;

struct BufferStreamProducer {
    buffer: Vec<u8>,
}

impl<D> StreamProducer<D> for BufferStreamProducer {
    type Item = u8;
    type Buffer = Option<u8>;

    fn poll_produce<'a>(
        self: Pin<&mut Self>,
        _: &mut Context<'_>,
        mut store: StoreContextMut<'a, D>,
        destination: Destination<'a, Self::Item, Self::Buffer>,
        _: bool,
    ) -> Poll<Result<StreamResult>> {
        let me = self.get_mut();
        let capacity = destination.remaining(store.as_context_mut());
        if capacity == Some(0) {
            Poll::Ready(Ok(StreamResult::Completed))
        } else {
            assert_eq!(capacity, Some(me.buffer.len()));
            let mut destination = destination.as_direct(store, me.buffer.len());
            destination.remaining().copy_from_slice(&me.buffer);
            destination.mark_written(me.buffer.len());

            Poll::Ready(Ok(StreamResult::Dropped))
        }
    }
}

struct BufferStreamConsumer {
    expected: Vec<u8>,
}

impl<D> StreamConsumer<D> for BufferStreamConsumer {
    type Item = u8;

    fn poll_consume(
        self: Pin<&mut Self>,
        _: &mut Context<'_>,
        mut store: StoreContextMut<D>,
        source: Source<Self::Item>,
        _: bool,
    ) -> Poll<Result<StreamResult>> {
        let me = self.get_mut();
        let available = source.remaining(store.as_context_mut());
        if available == 0 {
            Poll::Ready(Ok(StreamResult::Completed))
        } else {
            assert_eq!(available, me.expected.len());
            let mut source = source.as_direct(store);
            assert_eq!(&me.expected, source.remaining());
            source.mark_read(me.expected.len());

            Poll::Ready(Ok(StreamResult::Dropped))
        }
    }
}

struct ValueFutureProducer {
    value: u8,
}

impl<D> FutureProducer<D> for ValueFutureProducer {
    type Item = u8;

    fn poll_produce<'a>(
        self: Pin<&mut Self>,
        _: &mut Context<'_>,
        _: StoreContextMut<'a, D>,
        _: bool,
    ) -> Poll<Result<Option<Self::Item>>> {
        Poll::Ready(Ok(Some(self.value)))
    }
}

struct ValueFutureConsumer {
    expected: u8,
}

impl<D> FutureConsumer<D> for ValueFutureConsumer {
    type Item = u8;

    fn poll_consume(
        self: Pin<&mut Self>,
        _: &mut Context<'_>,
        store: StoreContextMut<D>,
        mut source: Source<'_, Self::Item>,
        _: bool,
    ) -> Poll<Result<()>> {
        let value = &mut None;
        source.read(store, value)?;
        assert_eq!(value.take(), Some(self.expected));
        Poll::Ready(Ok(()))
    }
}

struct DelayedStreamProducer<P> {
    inner: P,
    maybe_yield: Pin<Box<dyn Future<Output = ()> + Send>>,
}

impl<D, P: StreamProducer<D>> StreamProducer<D> for DelayedStreamProducer<P> {
    type Item = P::Item;
    type Buffer = P::Buffer;

    fn poll_produce<'a>(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        store: StoreContextMut<'a, D>,
        destination: Destination<'a, Self::Item, Self::Buffer>,
        finish: bool,
    ) -> Poll<Result<StreamResult>> {
        // SAFETY: We never move out of `self`.
        let maybe_yield = unsafe { &mut self.as_mut().get_unchecked_mut().maybe_yield };
        task::ready!(maybe_yield.as_mut().poll(cx));
        *maybe_yield = async {}.boxed();

        // SAFETY: This is a standard pin-projection, and we never move out
        // of `self`.
        let inner = unsafe { self.map_unchecked_mut(|v| &mut v.inner) };
        inner.poll_produce(cx, store, destination, finish)
    }
}

struct DelayedStreamConsumer<C> {
    inner: C,
    maybe_yield: Pin<Box<dyn Future<Output = ()> + Send>>,
}

impl<D, C: StreamConsumer<D>> StreamConsumer<D> for DelayedStreamConsumer<C> {
    type Item = C::Item;

    fn poll_consume(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        store: StoreContextMut<D>,
        source: Source<Self::Item>,
        finish: bool,
    ) -> Poll<Result<StreamResult>> {
        // SAFETY: We never move out of `self`.
        let maybe_yield = unsafe { &mut self.as_mut().get_unchecked_mut().maybe_yield };
        task::ready!(maybe_yield.as_mut().poll(cx));
        *maybe_yield = async {}.boxed();

        // SAFETY: This is a standard pin-projection, and we never move out
        // of `self`.
        let inner = unsafe { self.map_unchecked_mut(|v| &mut v.inner) };
        inner.poll_consume(cx, store, source, finish)
    }
}

struct DelayedFutureProducer<P> {
    inner: P,
    maybe_yield: Pin<Box<dyn Future<Output = ()> + Send>>,
}

impl<D, P: FutureProducer<D>> FutureProducer<D> for DelayedFutureProducer<P> {
    type Item = P::Item;

    fn poll_produce<'a>(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        store: StoreContextMut<'a, D>,
        finish: bool,
    ) -> Poll<Result<Option<Self::Item>>> {
        // SAFETY: We never move out of `self`.
        let maybe_yield = unsafe { &mut self.as_mut().get_unchecked_mut().maybe_yield };
        task::ready!(maybe_yield.as_mut().poll(cx));
        *maybe_yield = async {}.boxed();

        // SAFETY: This is a standard pin-projection, and we never move out
        // of `self`.
        let inner = unsafe { self.map_unchecked_mut(|v| &mut v.inner) };
        inner.poll_produce(cx, store, finish)
    }
}

struct DelayedFutureConsumer<C> {
    inner: C,
    maybe_yield: Pin<Box<dyn Future<Output = ()> + Send>>,
}

impl<D, C: FutureConsumer<D>> FutureConsumer<D> for DelayedFutureConsumer<C> {
    type Item = C::Item;

    fn poll_consume(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        store: StoreContextMut<D>,
        source: Source<'_, Self::Item>,
        finish: bool,
    ) -> Poll<Result<()>> {
        // SAFETY: We never move out of `self`.
        let maybe_yield = unsafe { &mut self.as_mut().get_unchecked_mut().maybe_yield };
        task::ready!(maybe_yield.as_mut().poll(cx));
        *maybe_yield = async {}.boxed();

        // SAFETY: This is a standard pin-projection, and we never move out
        // of `self`.
        let inner = unsafe { self.map_unchecked_mut(|v| &mut v.inner) };
        inner.poll_consume(cx, store, source, finish)
    }
}

struct ProcrastinatingStreamProducer<P>(P);

impl<D, P: StreamProducer<D>> StreamProducer<D> for ProcrastinatingStreamProducer<P> {
    type Item = P::Item;
    type Buffer = P::Buffer;

    fn poll_produce<'a>(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        store: StoreContextMut<'a, D>,
        destination: Destination<'a, Self::Item, Self::Buffer>,
        finish: bool,
    ) -> Poll<Result<StreamResult>> {
        if finish {
            // SAFETY: This is a standard pin-projection, and we never move out
            // of `self`.
            let producer = unsafe { self.map_unchecked_mut(|v| &mut v.0) };
            producer.poll_produce(cx, store, destination, true)
        } else {
            Poll::Pending
        }
    }
}

struct ProcrastinatingStreamConsumer<C>(C);

impl<D, C: StreamConsumer<D>> StreamConsumer<D> for ProcrastinatingStreamConsumer<C> {
    type Item = C::Item;

    fn poll_consume(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        store: StoreContextMut<D>,
        source: Source<'_, Self::Item>,
        finish: bool,
    ) -> Poll<Result<StreamResult>> {
        if finish {
            // SAFETY: This is a standard pin-projection, and we never move out
            // of `self`.
            let consumer = unsafe { self.map_unchecked_mut(|v| &mut v.0) };
            consumer.poll_consume(cx, store, source, true)
        } else {
            Poll::Pending
        }
    }
}

struct ProcrastinatingFutureProducer<P>(P);

impl<D, P: FutureProducer<D>> FutureProducer<D> for ProcrastinatingFutureProducer<P> {
    type Item = P::Item;

    fn poll_produce<'a>(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        store: StoreContextMut<'a, D>,
        finish: bool,
    ) -> Poll<Result<Option<Self::Item>>> {
        if finish {
            // SAFETY: This is a standard pin-projection, and we never move out
            // of `self`.
            let producer = unsafe { self.map_unchecked_mut(|v| &mut v.0) };
            producer.poll_produce(cx, store, true)
        } else {
            Poll::Pending
        }
    }
}

struct ProcrastinatingFutureConsumer<C>(C);

impl<D, C: FutureConsumer<D>> FutureConsumer<D> for ProcrastinatingFutureConsumer<C> {
    type Item = C::Item;

    fn poll_consume(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        store: StoreContextMut<D>,
        source: Source<'_, Self::Item>,
        finish: bool,
    ) -> Poll<Result<()>> {
        if finish {
            // SAFETY: This is a standard pin-projection, and we never move out
            // of `self`.
            let consumer = unsafe { self.map_unchecked_mut(|v| &mut v.0) };
            consumer.poll_consume(cx, store, source, true)
        } else {
            Poll::Pending
        }
    }
}

async fn yield_times(n: usize) {
    for _ in 0..n {
        tokio::task::yield_now().await;
    }
}

mod readiness {
    wasmtime::component::bindgen!({
        path: "wit",
        world: "readiness-guest",
    });
}

#[tokio::test]
pub async fn async_readiness() -> Result<()> {
    let component = test_programs_artifacts::ASYNC_READINESS_COMPONENT;

    let engine = Engine::new(&config())?;

    let component = make_component(&engine, &[component]).await?;

    let mut linker = Linker::new(&engine);

    wasmtime_wasi::p2::add_to_linker_async(&mut linker)?;

    let mut store = Store::new(
        &engine,
        Ctx {
            wasi: WasiCtxBuilder::new().inherit_stdio().build(),
            table: ResourceTable::default(),
            continue_: false,
        },
    );

    let readiness_guest =
        readiness::ReadinessGuest::instantiate_async(&mut store, &component, &linker).await?;
    let expected = vec![2u8, 4, 6, 8, 9];
    let rx = StreamReader::new(
        &mut store,
        DelayedStreamProducer {
            inner: BufferStreamProducer {
                buffer: expected.clone(),
            },
            maybe_yield: yield_times(10).boxed(),
        },
    );
    store
        .run_concurrent(async move |accessor| {
            let (rx, expected) = readiness_guest
                .local_local_readiness()
                .call_start(accessor, rx, expected)
                .await?;

            accessor.with(|access| {
                rx.pipe(
                    access,
                    DelayedStreamConsumer {
                        inner: BufferStreamConsumer { expected },
                        maybe_yield: yield_times(10).boxed(),
                    },
                )
            });

            Ok(())
        })
        .await?
}

#[tokio::test]
pub async fn async_poll_synchronous() -> Result<()> {
    test_run(&[test_programs_artifacts::ASYNC_POLL_SYNCHRONOUS_COMPONENT]).await
}

#[tokio::test]
pub async fn async_poll_stackless() -> Result<()> {
    test_run(&[test_programs_artifacts::ASYNC_POLL_STACKLESS_COMPONENT]).await
}

mod cancel {
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
    yield_::local::local::yield_::add_to_linker::<_, Ctx>(&mut linker, |ctx| ctx)?;

    let mut store = Store::new(
        &engine,
        Ctx {
            wasi: WasiCtxBuilder::new().inherit_stdio().build(),
            table: ResourceTable::default(),
            continue_: false,
        },
    );

    let cancel_host =
        cancel::CancelHost::instantiate_async(&mut store, &component, &linker).await?;
    store
        .run_concurrent(async move |accessor| {
            cancel_host
                .local_local_cancel()
                .call_run(accessor, mode, 100)
                .await?;
            Ok::<_, wasmtime::Error>(())
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
    ) -> impl Future<Output = Result<Self::Instance>>;

    fn call<'a>(
        accessor: &'a Accessor<Ctx, HasSelf<Ctx>>,
        instance: &'a Self::Instance,
        params: Self::Params,
    ) -> impl Future<Output = Result<Self::Result>> + Send + 'a;

    fn into_params(
        store: impl AsContextMut<Data = Ctx>,
        control: StreamReader<Control>,
        caller_stream: StreamReader<String>,
        caller_future1: FutureReader<String>,
        caller_future2: FutureReader<String>,
    ) -> Self::Params;

    fn from_result(
        store: impl AsContextMut<Data = Ctx>,
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
        store: impl AsContextMut<Data = Ctx>,
        component: &Component,
        linker: &Linker<Ctx>,
    ) -> Result<Self::Instance> {
        transmit::bindings::TransmitCallee::instantiate_async(store, &component, &linker).await
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
        _store: impl AsContextMut<Data = Ctx>,
        control: StreamReader<Control>,
        caller_stream: StreamReader<String>,
        caller_future1: FutureReader<String>,
        caller_future2: FutureReader<String>,
    ) -> Self::Params {
        (control, caller_stream, caller_future1, caller_future2)
    }

    fn from_result(
        _: impl AsContextMut<Data = Ctx>,
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
    ) -> Result<Self::Instance> {
        linker.instantiate_async(store, component).await
    }

    async fn call<'a>(
        accessor: &'a Accessor<Ctx, HasSelf<Ctx>>,
        instance: &'a Self::Instance,
        params: Self::Params,
    ) -> Result<Self::Result> {
        let exchange_function = accessor.with(|mut store| {
            let transmit_instance = instance
                .get_export_index(store.as_context_mut(), None, "local:local/transmit")
                .ok_or_else(|| format_err!("can't find `local:local/transmit` in instance"))?;
            let exchange_function = instance
                .get_export_index(store.as_context_mut(), Some(&transmit_instance), "exchange")
                .ok_or_else(|| format_err!("can't find `exchange` in instance"))?;
            instance
                .get_func(store.as_context_mut(), exchange_function)
                .ok_or_else(|| format_err!("can't find `exchange` in instance"))
        })?;

        let mut results = vec![Val::Bool(false)];
        exchange_function
            .call_concurrent(accessor, &params, &mut results)
            .await?;
        Ok(results.pop().unwrap())
    }

    fn into_params(
        mut store: impl AsContextMut<Data = Ctx>,
        control: StreamReader<Control>,
        caller_stream: StreamReader<String>,
        caller_future1: FutureReader<String>,
        caller_future2: FutureReader<String>,
    ) -> Self::Params {
        vec![
            control.try_into_stream_any(&mut store).unwrap().into(),
            caller_stream
                .try_into_stream_any(&mut store)
                .unwrap()
                .into(),
            caller_future1
                .try_into_future_any(&mut store)
                .unwrap()
                .into(),
            caller_future2
                .try_into_future_any(&mut store)
                .unwrap()
                .into(),
        ]
    }

    fn from_result(
        _store: impl AsContextMut<Data = Ctx>,
        result: Self::Result,
    ) -> Result<(
        StreamReader<String>,
        FutureReader<String>,
        FutureReader<String>,
    )> {
        let Val::Tuple(fields) = result else {
            unreachable!()
        };
        let mut fields = fields.into_iter();
        let Val::Stream(stream) = fields.next().unwrap() else {
            unreachable!()
        };
        let Val::Future(future1) = fields.next().unwrap() else {
            unreachable!()
        };
        let Val::Future(future2) = fields.next().unwrap() else {
            unreachable!()
        };
        let stream = StreamReader::try_from_stream_any(stream).unwrap();
        let future1 = FutureReader::try_from_future_any(future1).unwrap();
        let future2 = FutureReader::try_from_future_any(future2).unwrap();
        Ok((stream, future1, future2))
    }
}

async fn test_transmit(component: &str) -> Result<()> {
    test_transmit_with::<StaticTransmitTest>(component).await?;
    test_transmit_with::<DynamicTransmitTest>(component).await
}

async fn test_transmit_with<Test: TransmitTest + 'static>(component: &str) -> Result<()> {
    let engine = Engine::new(&config())?;

    let component = make_component(&engine, &[component]).await?;

    let mut linker = Linker::new(&engine);

    wasmtime_wasi::p2::add_to_linker_async(&mut linker)?;

    let mut store = Store::new(
        &engine,
        Ctx {
            wasi: WasiCtxBuilder::new().inherit_stdio().build(),
            table: ResourceTable::default(),
            continue_: false,
        },
    );

    let test = Test::instantiate(&mut store, &component, &linker).await?;

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
    let control_rx = StreamReader::new(&mut store, PipeProducer::new(control_rx));
    let (mut caller_stream_tx, caller_stream_rx) = mpsc::channel(1);
    let caller_stream_rx = StreamReader::new(&mut store, PipeProducer::new(caller_stream_rx));
    let (caller_future1_tx, caller_future1_rx) = oneshot::channel();
    let caller_future1_rx = FutureReader::new(&mut store, OneshotProducer::new(caller_future1_rx));
    let (_, caller_future2_rx) = oneshot::channel();
    let caller_future2_rx = FutureReader::new(&mut store, OneshotProducer::new(caller_future2_rx));
    let (callee_future1_tx, callee_future1_rx) = oneshot::channel();
    let (callee_stream_tx, callee_stream_rx) = mpsc::channel(1);
    store
        .run_concurrent(async |accessor| {
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

            let params = accessor.with(|s| {
                Test::into_params(
                    s,
                    control_rx,
                    caller_stream_rx,
                    caller_future1_rx,
                    caller_future2_rx,
                )
            });

            futures.push(
                Test::call(accessor, &test, params)
                    .map(|v| v.map(Event::Result))
                    .boxed(),
            );

            while let Some(event) = futures.try_next().await? {
                match event {
                    Event::Result(result) => {
                        accessor.with(|mut store| {
                            let (callee_stream_rx, callee_future1_rx, _) =
                                Test::from_result(&mut store, result)?;
                            callee_stream_rx.pipe(
                                &mut store,
                                PipeConsumer::new(callee_stream_tx.take().unwrap()),
                            );
                            callee_future1_rx.pipe(
                                &mut store,
                                OneshotConsumer::new(callee_future1_tx.take().unwrap()),
                            );
                            wasmtime::error::Ok(())
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

            wasmtime::error::Ok(())
        })
        .await??;
    Ok(())
}

mod synchronous_transmit {
    wasmtime::component::bindgen!({
        path: "wit",
        world: "synchronous-transmit-guest",
    });
}

#[tokio::test]
pub async fn async_cancel_transmit() -> Result<()> {
    test_synchronous_transmit(
        test_programs_artifacts::ASYNC_CANCEL_TRANSMIT_COMPONENT,
        true,
    )
    .await
}

#[tokio::test]
pub async fn async_synchronous_transmit() -> Result<()> {
    test_synchronous_transmit(
        test_programs_artifacts::ASYNC_SYNCHRONOUS_TRANSMIT_COMPONENT,
        false,
    )
    .await
}

async fn test_synchronous_transmit(component: &str, procrastinate: bool) -> Result<()> {
    let engine = Engine::new(&config())?;

    let component = make_component(&engine, &[component]).await?;

    let mut linker = Linker::new(&engine);

    wasmtime_wasi::p2::add_to_linker_async(&mut linker)?;

    let mut store = Store::new(
        &engine,
        Ctx {
            wasi: WasiCtxBuilder::new().inherit_stdio().build(),
            table: ResourceTable::default(),
            continue_: false,
        },
    );

    let instance = linker.instantiate_async(&mut store, &component).await?;
    let guest = synchronous_transmit::SynchronousTransmitGuest::new(&mut store, &instance)?;
    let stream_expected = vec![2u8, 4, 6, 8, 9];
    let producer = DelayedStreamProducer {
        inner: BufferStreamProducer {
            buffer: stream_expected.clone(),
        },
        maybe_yield: yield_times(10).boxed(),
    };
    let stream = if procrastinate {
        StreamReader::new(&mut store, ProcrastinatingStreamProducer(producer))
    } else {
        StreamReader::new(&mut store, producer)
    };
    let future_expected = 10;
    let producer = DelayedFutureProducer {
        inner: ValueFutureProducer {
            value: future_expected,
        },
        maybe_yield: yield_times(10).boxed(),
    };
    let future = if procrastinate {
        FutureReader::new(&mut store, ProcrastinatingFutureProducer(producer))
    } else {
        FutureReader::new(&mut store, producer)
    };
    store
        .run_concurrent(async move |accessor| {
            let (stream, stream_expected, future, future_expected) = guest
                .local_local_synchronous_transmit()
                .call_start(accessor, stream, stream_expected, future, future_expected)
                .await?;

            accessor.with(|mut access| {
                let consumer = DelayedStreamConsumer {
                    inner: BufferStreamConsumer {
                        expected: stream_expected,
                    },
                    maybe_yield: yield_times(10).boxed(),
                };
                if procrastinate {
                    stream.pipe(&mut access, ProcrastinatingStreamConsumer(consumer));
                } else {
                    stream.pipe(&mut access, consumer);
                }
                let consumer = DelayedFutureConsumer {
                    inner: ValueFutureConsumer {
                        expected: future_expected,
                    },
                    maybe_yield: yield_times(10).boxed(),
                };
                if procrastinate {
                    future.pipe(access, ProcrastinatingFutureConsumer(consumer));
                } else {
                    future.pipe(access, consumer);
                }
            });

            Ok(())
        })
        .await?
}
