use crate::error::ptr::{MutPtr, OwnedPtr, SharedPtr};
use crate::error::{
    ConcreteError, DynError, ErrorExt, OomOrDynErrorMut, OomOrDynErrorRef, OutOfMemory,
};
use core::{any::TypeId, fmt, ptr::NonNull};
use std_alloc::boxed::Box;

/// A vtable containing the `ErrorExt` methods for some type `T`.
///
/// This is used to create thin-pointer equivalents of `Box<dyn ErrorExt>`,
/// `&dyn ErrorExt`, and `&mut ErrorExt`, which would all otherwise be two words
/// in size.
///
/// # Safety
///
/// The safety contract for all vtable functions is the same:
///
/// * `SharedPtr<'_, DynError>`s must be valid for reading a `ConcreteError<T>`,
///   `MutPtr<'_, DynError>`s must additionally be valid for writing a
///   `ConcreteError<T>`, and `OwnedPtr<DynError>`s must additionally be valid
///   to deallocate with `ConcreteError<T>`'s layout.
///
/// * If a `OomOrDynError{Ref,Mut}` return value contains a `{Shared,Mut}Ptr<'_,
///   DynError>`, it must be valid for reading (and, in the case of `MutPtr`,
///   writing) `DynError`s.
#[repr(C)]
pub(crate) struct Vtable {
    pub(crate) display: unsafe fn(SharedPtr<'_, DynError>, &mut fmt::Formatter<'_>) -> fmt::Result,
    pub(crate) debug: unsafe fn(SharedPtr<'_, DynError>, &mut fmt::Formatter<'_>) -> fmt::Result,
    pub(crate) source: unsafe fn(SharedPtr<'_, DynError>) -> Option<OomOrDynErrorRef<'_>>,
    pub(crate) source_mut: unsafe fn(MutPtr<'_, DynError>) -> Option<OomOrDynErrorMut<'_>>,
    pub(crate) is: unsafe fn(SharedPtr<'_, DynError>, TypeId) -> bool,
    pub(crate) as_dyn_core_error:
        unsafe fn(SharedPtr<'_, DynError>) -> &(dyn core::error::Error + Send + Sync + 'static),
    pub(crate) into_boxed_dyn_core_error:
        unsafe fn(
            OwnedPtr<DynError>,
        )
            -> Result<Box<dyn core::error::Error + Send + Sync + 'static>, OutOfMemory>,
    pub(crate) drop_and_deallocate: unsafe fn(OwnedPtr<DynError>),

    /// Additional safety requirement: the `NonNull<u8>` pointer must be valid
    /// for writing a `T`.
    ///
    /// Upon successful return, a `T` will have been written to that memory
    /// block.
    pub(crate) downcast: unsafe fn(OwnedPtr<DynError>, TypeId, NonNull<u8>),
}

impl Vtable {
    /// Get the `Vtable` of the `E: ErrorExt` type parameter.
    pub(crate) fn of<E>() -> &'static Self
    where
        E: ErrorExt,
    {
        &Vtable {
            display: display::<E>,
            debug: debug::<E>,
            source: source::<E>,
            source_mut: source_mut::<E>,
            is: is::<E>,
            as_dyn_core_error: as_dyn_core_error::<E>,
            into_boxed_dyn_core_error: into_boxed_dyn_core_error::<E>,
            drop_and_deallocate: drop_and_deallocate::<E>,
            downcast: downcast::<E>,
        }
    }
}

unsafe fn display<E>(error: SharedPtr<'_, DynError>, f: &mut fmt::Formatter<'_>) -> fmt::Result
where
    E: ErrorExt,
{
    let error = error.cast::<ConcreteError<E>>();
    // Safety: implied by all vtable functions' safety contract.
    let error = unsafe { error.as_ref() };
    fmt::Display::fmt(error.error.ext_as_dyn_core_error(), f)
}

unsafe fn debug<E>(error: SharedPtr<'_, DynError>, f: &mut fmt::Formatter<'_>) -> fmt::Result
where
    E: ErrorExt,
{
    let error = error.cast::<ConcreteError<E>>();
    // Safety: implied by all vtable functions' safety contract.
    let error = unsafe { error.as_ref() };
    fmt::Debug::fmt(error.error.ext_as_dyn_core_error(), f)
}

unsafe fn source<E>(error: SharedPtr<'_, DynError>) -> Option<OomOrDynErrorRef<'_>>
where
    E: ErrorExt,
{
    let error = error.cast::<ConcreteError<E>>();
    // Safety: implied by all vtable functions' safety contract.
    let error = unsafe { error.as_ref() };
    error.error.ext_source()
}

unsafe fn source_mut<E>(error: MutPtr<'_, DynError>) -> Option<OomOrDynErrorMut<'_>>
where
    E: ErrorExt,
{
    let mut error = error.cast::<ConcreteError<E>>();
    // Safety: implied by all vtable functions' safety contract.
    let error = unsafe { error.as_mut() };
    error.error.ext_source_mut()
}

unsafe fn is<E>(error: SharedPtr<'_, DynError>, type_id: TypeId) -> bool
where
    E: ErrorExt,
{
    let error = error.cast::<ConcreteError<E>>();
    // Safety: implied by all vtable functions' safety contract.
    let error = unsafe { error.as_ref() };
    error.error.ext_is(type_id)
}

unsafe fn as_dyn_core_error<E>(
    error: SharedPtr<'_, DynError>,
) -> &(dyn core::error::Error + Send + Sync + 'static)
where
    E: ErrorExt,
{
    let error = error.cast::<ConcreteError<E>>();
    // Safety: implied by all vtable functions' safety contract.
    let error = unsafe { error.as_ref() };
    error.error.ext_as_dyn_core_error()
}

unsafe fn into_boxed_dyn_core_error<E>(
    error: OwnedPtr<DynError>,
) -> Result<Box<dyn core::error::Error + Send + Sync + 'static>, OutOfMemory>
where
    E: ErrorExt,
{
    let error = error.cast::<ConcreteError<E>>();
    // Safety: implied by all vtable functions' safety contract.
    let error = unsafe { error.into_box() };
    error.error.ext_into_boxed_dyn_core_error()
}

unsafe fn drop_and_deallocate<E>(error: OwnedPtr<DynError>)
where
    E: ErrorExt,
{
    let error = error.cast::<ConcreteError<E>>();
    // Safety: implied by all vtable functions' safety contract.
    let _ = unsafe { error.into_box() };
}

unsafe fn downcast<E>(error: OwnedPtr<DynError>, type_id: TypeId, ret_ptr: NonNull<u8>)
where
    E: ErrorExt,
{
    let error = error.cast::<ConcreteError<E>>();
    // Safety: implied by all vtable functions' safety contract.
    let mut error = unsafe { error.into_box() };

    if error.error.ext_is(type_id) {
        // Safety: Implied by `downcast`'s additional safety safety requirement.
        unsafe {
            error.error.ext_move(ret_ptr);
        }
    } else {
        let source = error
            .error
            .ext_take_source()
            .expect("must have a source up the chain if `E` is not our target type");
        // Safety: implied by downcast's additional safety requirement.
        unsafe {
            source.downcast(type_id, ret_ptr);
        }
    }
}
