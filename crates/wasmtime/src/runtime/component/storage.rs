use crate::ValRaw;
use core::mem::{self, MaybeUninit};
use core::slice;

fn assert_raw_slice_compat<T>() {
    assert!(mem::size_of::<T>() % mem::size_of::<ValRaw>() == 0);
    assert!(mem::align_of::<T>() == mem::align_of::<ValRaw>());
}

/// Converts a `<T as ComponentType>::Lower` representation to a slice of
/// `ValRaw`.
pub unsafe fn storage_as_slice<T>(storage: &T) -> &[ValRaw] {
    assert_raw_slice_compat::<T>();

    slice::from_raw_parts(
        (storage as *const T).cast(),
        mem::size_of_val(storage) / mem::size_of::<ValRaw>(),
    )
}

/// Same as `storage_as_slice`, but mutable.
pub unsafe fn storage_as_slice_mut<T>(storage: &mut T) -> &mut [ValRaw] {
    assert_raw_slice_compat::<T>();

    slice::from_raw_parts_mut(
        (storage as *mut T).cast(),
        mem::size_of_val(storage) / mem::size_of::<ValRaw>(),
    )
}

/// Same as `storage_as_slice`, but in reverse and mutable.
pub unsafe fn slice_to_storage_mut<T>(slice: &mut [MaybeUninit<ValRaw>]) -> &mut MaybeUninit<T> {
    assert_raw_slice_compat::<T>();

    // This is an actual runtime assertion which if performance calls for we may
    // need to relax to a debug assertion. This notably tries to ensure that we
    // stay within the bounds of the number of actual values given rather than
    // reading past the end of an array. This shouldn't actually trip unless
    // there's a bug in Wasmtime though.
    assert!(
        mem::size_of_val(slice) >= mem::size_of::<T>(),
        "needed {}; got {}",
        mem::size_of::<T>(),
        mem::size_of_val(slice)
    );

    &mut *slice.as_mut_ptr().cast()
}

/// Same as `storage_as_slice`, but in reverse
#[cfg(feature = "component-model-async")]
pub unsafe fn slice_to_storage<T>(slice: &[ValRaw]) -> &T {
    assert_raw_slice_compat::<T>();

    // This is an actual runtime assertion which if performance calls for we may
    // need to relax to a debug assertion. This notably tries to ensure that we
    // stay within the bounds of the number of actual values given rather than
    // reading past the end of an array. This shouldn't actually trip unless
    // there's a bug in Wasmtime though.
    assert!(
        mem::size_of_val(slice) >= mem::size_of::<T>(),
        "needed {}; got {}",
        mem::size_of::<T>(),
        mem::size_of_val(slice)
    );

    &*slice.as_ptr().cast()
}
