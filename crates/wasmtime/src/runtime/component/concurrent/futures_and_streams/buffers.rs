use std::{any::TypeId, marker::PhantomData};

/// Trait representing a buffer from which items may be moved (i.e. ownership
/// transferred).
///
/// SAFETY: `Self::take` must verify the requested number of items are
/// available, must pass a `TypeId` corresponding to the type of items being
/// taken, and must `mem::forget` those items after the call to `fun` returns.
#[doc(hidden)]
pub unsafe trait TakeBuffer {
    /// Take ownership of the specified number of items.
    ///
    /// The items are passed to `fun` as a raw pointer which may be cast to the
    /// type indicated by the specified `TypeId`.
    fn take(&mut self, count: usize, fun: &mut dyn FnMut(TypeId, *const u8));
}

/// Trait representing a buffer which may be written to a `StreamWriter`.
#[doc(hidden)]
pub trait WriteBuffer<T>: TakeBuffer + Send + Sync + 'static {
    /// Slice of items remaining to be read.
    fn remaining(&self) -> &[T];
    /// Skip and drop the specified number of items.
    fn skip(&mut self, count: usize);
}

/// Trait representing a buffer which may be used to read from a `StreamReader`.
#[doc(hidden)]
pub trait ReadBuffer<T>: Send + Sync + 'static {
    /// Move the specified items into this buffer.
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I);
    /// Number of items which may be read before this buffer is full.
    fn remaining_capacity(&self) -> usize;
    /// Move (i.e. take ownership of) the specified items into this buffer.
    ///
    /// This will panic if the specified `input` item type does not match `T`.
    fn move_from(&mut self, input: &mut dyn TakeBuffer, count: usize);
}

/// A `WriteBuffer` implementation, backed by a `Vec`.
pub struct VecBuffer<T> {
    _phantom: PhantomData<T>,
}
