use super::TryNew;
use std_alloc::sync::Arc;
use wasmtime_error::OutOfMemory;

/// XXX: Stable Rust doesn't actually give us any method to build fallible
/// allocation for `Arc<T>`, so this is only actually fallible when using
/// nightly Rust and setting `RUSTFLAGS="--cfg arc_try_new"`.
impl<T> TryNew for Arc<T> {
    type Value = T;

    #[inline]
    fn try_new(value: T) -> Result<Self, OutOfMemory>
    where
        Self: Sized,
    {
        #[cfg(arc_try_new)]
        return Arc::try_new(value).map_err(|_| {
            // We don't have access to the exact size of the inner `Arc`
            // allocation, but (at least at one point) it was made up of a
            // strong ref count, a weak ref count, and the inner value.
            let bytes = core::mem::size_of::<(usize, usize, T)>();
            OutOfMemory::new(bytes)
        });

        #[cfg(not(arc_try_new))]
        return Ok(Arc::new(value));
    }
}

#[cfg(test)]
mod test {
    use super::{Arc, TryNew};

    #[test]
    fn try_new() {
        Arc::try_new(4).unwrap();
    }
}
