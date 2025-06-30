use crate::Uninhabited;
use crate::component::func::{ComponentType, LiftContext, LowerContext};
use crate::component::{Instance, Val};
use crate::runtime::vm::VMStore;
use anyhow::{Result, anyhow};
use core::future::Future;
use core::marker::PhantomData;
use core::pin::pin;
use core::task::{Context, Poll, Waker};
use wasmtime_environ::component::{InterfaceType, RuntimeComponentInstanceIndex};

impl Instance {
    pub(crate) fn poll_and_block<R: Send + Sync + 'static>(
        self,
        _store: &mut dyn VMStore,
        future: impl Future<Output = Result<R>> + Send + 'static,
        _caller_instance: RuntimeComponentInstanceIndex,
    ) -> Result<R> {
        match pin!(future).poll(&mut Context::from_waker(Waker::noop())) {
            Poll::Ready(result) => result,
            Poll::Pending => {
                // This should be unreachable; if we trap here, it indicates a
                // bug in Wasmtime rather than in the guest.
                Err(anyhow!(
                    "async-lowered import should have failed validation \
                     when `component-model-async` feature disabled"
                ))
            }
        }
    }
}

pub(crate) fn lower_future_to_index<U>(
    _rep: u32,
    _cx: &mut LowerContext<'_, U>,
    _ty: InterfaceType,
) -> Result<u32> {
    // This should be unreachable; if we trap here, it indicates a bug in
    // Wasmtime rather than in the guest.
    Err(anyhow!(
        "use of `future` should have failed validation \
         when `component-model-async` feature disabled"
    ))
}

pub(crate) fn lower_stream_to_index<U>(
    _rep: u32,
    _cx: &mut LowerContext<'_, U>,
    _ty: InterfaceType,
) -> Result<u32> {
    // This should be unreachable; if we trap here, it indicates a bug in
    // Wasmtime rather than in the guest.
    Err(anyhow!(
        "use of `stream` should have failed validation \
         when `component-model-async` feature disabled"
    ))
}

pub(crate) fn lower_error_context_to_index<U>(
    _rep: u32,
    _cx: &mut LowerContext<'_, U>,
    _ty: InterfaceType,
) -> Result<u32> {
    // This should be unreachable; if we trap here, it indicates a bug in
    // Wasmtime rather than in the guest.
    Err(anyhow!(
        "use of `error-context` should have failed validation \
         when `component-model-async` feature disabled"
    ))
}

pub struct ErrorContext(Uninhabited);

impl ErrorContext {
    pub(crate) fn into_val(self) -> Val {
        match self.0 {}
    }

    pub(crate) fn linear_lift_from_flat(
        _cx: &mut LiftContext<'_>,
        _ty: InterfaceType,
        _src: &<u32 as ComponentType>::Lower,
    ) -> Result<Self> {
        // This should be unreachable; if we trap here, it indicates a bug in
        // Wasmtime rather than in the guest.
        Err(anyhow!(
            "use of `error-context` should have failed validation \
             when `component-model-async` feature disabled"
        ))
    }

    pub(crate) fn linear_lift_from_memory(
        _cx: &mut LiftContext<'_>,
        _ty: InterfaceType,
        _bytes: &[u8],
    ) -> Result<Self> {
        // This should be unreachable; if we trap here, it indicates a bug in
        // Wasmtime rather than in the guest.
        Err(anyhow!(
            "use of `error-context` should have failed validation \
             when `component-model-async` feature disabled"
        ))
    }
}

pub struct HostStream<P> {
    uninhabited: Uninhabited,
    _phantom: PhantomData<P>,
}

impl<P> HostStream<P> {
    pub(crate) fn into_val(self) -> Val {
        match self.uninhabited {}
    }

    pub(crate) fn linear_lift_from_flat(
        _cx: &mut LiftContext<'_>,
        _ty: InterfaceType,
        _src: &<u32 as ComponentType>::Lower,
    ) -> Result<Self> {
        // This should be unreachable; if we trap here, it indicates a bug in
        // Wasmtime rather than in the guest.
        Err(anyhow!(
            "use of `stream` should have failed validation \
             when `component-model-async` feature disabled"
        ))
    }

    pub(crate) fn linear_lift_from_memory(
        _cx: &mut LiftContext<'_>,
        _ty: InterfaceType,
        _bytes: &[u8],
    ) -> Result<Self> {
        // This should be unreachable; if we trap here, it indicates a bug in
        // Wasmtime rather than in the guest.
        Err(anyhow!(
            "use of `stream` should have failed validation \
             when `component-model-async` feature disabled"
        ))
    }
}

pub struct HostFuture<P> {
    uninhabited: Uninhabited,
    _phantom: PhantomData<P>,
}

impl<P> HostFuture<P> {
    pub(crate) fn into_val(self) -> Val {
        match self.uninhabited {}
    }

    pub(crate) fn linear_lift_from_flat(
        _cx: &mut LiftContext<'_>,
        _ty: InterfaceType,
        _src: &<u32 as ComponentType>::Lower,
    ) -> Result<Self> {
        // This should be unreachable; if we trap here, it indicates a bug in
        // Wasmtime rather than in the guest.
        Err(anyhow!(
            "use of `future` should have failed validation \
             when `component-model-async` feature disabled"
        ))
    }

    pub(crate) fn linear_lift_from_memory(
        _cx: &mut LiftContext<'_>,
        _ty: InterfaceType,
        _bytes: &[u8],
    ) -> Result<Self> {
        // This should be unreachable; if we trap here, it indicates a bug in
        // Wasmtime rather than in the guest.
        Err(anyhow!(
            "use of `future` should have failed validation \
             when `component-model-async` feature disabled"
        ))
    }
}
