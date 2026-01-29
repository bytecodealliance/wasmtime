use crate::component::func::{LiftContext, LowerContext};
use crate::component::matching::InstanceType;
use crate::component::{ComponentType, Lift, Lower, RuntimeInstance, Val};
use crate::store::StoreOpaque;
use crate::{Result, bail, error::format_err};
use core::convert::Infallible;
use core::mem::MaybeUninit;
use wasmtime_environ::component::{CanonicalAbiInfo, InterfaceType};

pub enum ConcurrentState {}

fn should_have_failed_validation<T>(what: &str) -> Result<T> {
    // This should be unreachable; if we trap here, it indicates a
    // bug in Wasmtime rather than in the guest.
    Err(format_err!(
        "{what} should have failed validation \
         when `component-model-async` feature disabled"
    ))
}

pub(crate) fn lower_error_context_to_index<U>(
    _rep: u32,
    _cx: &mut LowerContext<'_, U>,
    _ty: InterfaceType,
) -> Result<u32> {
    should_have_failed_validation("use of `error-context`")
}

pub struct ErrorContext(Infallible);

impl ErrorContext {
    pub(crate) fn into_val(self) -> Val {
        match self.0 {}
    }

    pub(crate) fn linear_lift_from_flat(
        _cx: &mut LiftContext<'_>,
        _ty: InterfaceType,
        _src: &<u32 as ComponentType>::Lower,
    ) -> Result<Self> {
        should_have_failed_validation("use of `error-context`")
    }

    pub(crate) fn linear_lift_from_memory(
        _cx: &mut LiftContext<'_>,
        _ty: InterfaceType,
        _bytes: &[u8],
    ) -> Result<Self> {
        should_have_failed_validation("use of `error-context`")
    }
}

#[derive(PartialEq, Clone, Debug)]
pub struct FutureAny(Infallible);

unsafe impl ComponentType for FutureAny {
    type Lower = <u32 as ComponentType>::Lower;
    const ABI: CanonicalAbiInfo = CanonicalAbiInfo::SCALAR4;

    fn typecheck(_ty: &InterfaceType, _types: &InstanceType<'_>) -> Result<()> {
        bail!("support for component-model-async disabled at compile time")
    }
}

unsafe impl Lift for FutureAny {
    fn linear_lift_from_flat(
        _cx: &mut LiftContext<'_>,
        _ty: InterfaceType,
        _src: &Self::Lower,
    ) -> Result<Self> {
        bail!("support for component-model-async disabled at compile time")
    }

    fn linear_lift_from_memory(
        _cx: &mut LiftContext<'_>,
        _ty: InterfaceType,
        _bytes: &[u8],
    ) -> Result<Self> {
        bail!("support for component-model-async disabled at compile time")
    }
}

unsafe impl Lower for FutureAny {
    fn linear_lower_to_flat<T>(
        &self,
        _cx: &mut LowerContext<'_, T>,
        _ty: InterfaceType,
        _dst: &mut MaybeUninit<Self::Lower>,
    ) -> Result<()> {
        match self.0 {}
    }

    fn linear_lower_to_memory<T>(
        &self,
        _cx: &mut LowerContext<'_, T>,
        _ty: InterfaceType,
        _offset: usize,
    ) -> Result<()> {
        match self.0 {}
    }
}

#[derive(PartialEq, Clone, Debug)]
pub struct StreamAny(Infallible);

unsafe impl ComponentType for StreamAny {
    type Lower = <u32 as ComponentType>::Lower;
    const ABI: CanonicalAbiInfo = CanonicalAbiInfo::SCALAR4;

    fn typecheck(_ty: &InterfaceType, _types: &InstanceType<'_>) -> Result<()> {
        bail!("support for component-model-async disabled at compile time")
    }
}

unsafe impl Lift for StreamAny {
    fn linear_lift_from_flat(
        _cx: &mut LiftContext<'_>,
        _ty: InterfaceType,
        _src: &Self::Lower,
    ) -> Result<Self> {
        bail!("support for component-model-async disabled at compile time")
    }

    fn linear_lift_from_memory(
        _cx: &mut LiftContext<'_>,
        _ty: InterfaceType,
        _bytes: &[u8],
    ) -> Result<Self> {
        bail!("support for component-model-async disabled at compile time")
    }
}

unsafe impl Lower for StreamAny {
    fn linear_lower_to_flat<T>(
        &self,
        _cx: &mut LowerContext<'_, T>,
        _ty: InterfaceType,
        _dst: &mut MaybeUninit<Self::Lower>,
    ) -> Result<()> {
        match self.0 {}
    }

    fn linear_lower_to_memory<T>(
        &self,
        _cx: &mut LowerContext<'_, T>,
        _ty: InterfaceType,
        _offset: usize,
    ) -> Result<()> {
        match self.0 {}
    }
}

impl StoreOpaque {
    pub(crate) fn enter_sync_call(
        &mut self,
        _guest_caller: Option<RuntimeInstance>,
        _callee_async: bool,
        _callee: RuntimeInstance,
    ) -> Result<()> {
        // If we've reached here, the user somehow managed to enable the
        // `component-model-async` runtime config feature without enabling the
        // corresponding compile-time feature.
        unreachable!()
    }

    pub(crate) fn exit_sync_call(&mut self, _guest_caller: bool) -> Result<()> {
        // See comment in `enter_sync_call`
        unreachable!()
    }

    pub(crate) fn check_blocking(&mut self) -> crate::Result<()> {
        Ok(())
    }

    pub(crate) fn may_enter(&mut self, instance: RuntimeInstance) -> bool {
        self.may_enter_at_all(instance)
    }
}
