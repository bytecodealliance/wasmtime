use bytes::{Bytes, BytesMut};
use std::io::Cursor;
use std::mem::{self, MaybeUninit};
use std::ptr;
use std::slice;
use std::vec::Vec;

// Inner module here to restrict possible readers of the fields of
// `UntypedWriteBuffer`.
pub use untyped::*;
mod untyped {
    use super::WriteBuffer;
    use std::any::TypeId;
    use std::marker;
    use std::mem;

    /// Helper structure to type-erase the `T` in `WriteBuffer<T>`.
    ///
    /// This is constructed with a `&mut dyn WriteBuffer<T>` and then can only
    /// be viewed as `&mut dyn WriteBuffer<T>` as well. The `T`, however, is
    /// carried through methods rather than the struct itself.
    ///
    /// Note that this structure has a lifetime `'a` which forces an active
    /// borrow on the original buffer passed in.
    pub struct UntypedWriteBuffer<'a> {
        element_type_id: TypeId,
        buf: *mut dyn WriteBuffer<()>,
        _marker: marker::PhantomData<&'a mut dyn WriteBuffer<()>>,
    }

    /// Helper structure to transmute between `WriteBuffer<T>` and
    /// `WriteBuffer<()>`.
    union ReinterpretWriteBuffer<T> {
        typed: *mut dyn WriteBuffer<T>,
        untyped: *mut dyn WriteBuffer<()>,
    }

    impl<'a> UntypedWriteBuffer<'a> {
        /// Creates a new `UntypedWriteBuffer` from the `buf` provided.
        ///
        /// The returned value can be used with the `get_mut` method to get the
        /// original write buffer back.
        pub fn new<T: 'static>(buf: &'a mut dyn WriteBuffer<T>) -> UntypedWriteBuffer<'a> {
            UntypedWriteBuffer {
                element_type_id: TypeId::of::<T>(),
                // SAFETY: this is `unsafe` due to reading union fields. That
                // is safe here because `typed` and `untyped` have the same size
                // and we're otherwise reinterpreting a raw pointer with a type
                // parameter to one without one.
                buf: unsafe {
                    let r = ReinterpretWriteBuffer { typed: buf };
                    assert_eq!(mem::size_of_val(&r.typed), mem::size_of_val(&r.untyped));
                    r.untyped
                },
                _marker: marker::PhantomData,
            }
        }

        /// Acquires the underyling `WriteBuffer<T>` this was created with.
        ///
        /// # Panics
        ///
        /// Panics if `T` does not match the type that this was created with.
        pub fn get_mut<T: 'static>(&mut self) -> &mut dyn WriteBuffer<T> {
            assert_eq!(self.element_type_id, TypeId::of::<T>());
            // SAFETY: the `T` has been checked with `TypeId` and this
            // structure also is proof of valid existence of the original
            // `&mut WriteBuffer<T>`, so taking the raw pointer back to a safe
            // reference is valid.
            unsafe { &mut *ReinterpretWriteBuffer { untyped: self.buf }.typed }
        }
    }
}

/// Trait representing a buffer which may be written to a `StreamWriter`.
#[doc(hidden)]
pub trait WriteBuffer<T>: Send + Sync + 'static {
    /// Slice of items remaining to be read.
    fn remaining(&self) -> &[T];
    /// Skip and drop the specified number of items.
    fn skip(&mut self, count: usize);
    /// Take ownership of the specified number of items.
    ///
    /// The items are passed to `fun` as a raw pointer.
    fn take(&mut self, count: usize, fun: &mut dyn FnMut(*const T));
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
    fn move_from(&mut self, input: &mut dyn WriteBuffer<T>, count: usize);
}

pub(super) struct Extender<'a, B>(pub(super) &'a mut B);

impl<T, B: ReadBuffer<T>> Extend<T> for Extender<'_, B> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        self.0.extend(iter)
    }
}

impl<T: Send + Sync + 'static> WriteBuffer<T> for Option<T> {
    fn remaining(&self) -> &[T] {
        if let Some(me) = self {
            slice::from_ref(me)
        } else {
            &[]
        }
    }

    fn skip(&mut self, count: usize) {
        match count {
            0 => {}
            1 => {
                assert!(self.is_some());
                *self = None;
            }
            _ => panic!("cannot skip more than {} item(s)", self.remaining().len()),
        }
    }

    fn take(&mut self, count: usize, fun: &mut dyn FnMut(*const T)) {
        match count {
            0 => fun(ptr::null_mut()),
            1 => {
                assert!(self.is_some());
                fun(self.remaining().as_ptr());
                mem::forget(self.take());
            }
            _ => panic!("cannot forget more than {} item(s)", self.remaining().len()),
        }
    }
}

impl<T: Send + Sync + 'static> ReadBuffer<T> for Option<T> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        let mut iter = iter.into_iter();
        if self.is_none() {
            *self = iter.next();
        }
        assert!(iter.next().is_none());
    }

    fn remaining_capacity(&self) -> usize {
        if self.is_some() { 0 } else { 1 }
    }

    fn move_from(&mut self, input: &mut dyn WriteBuffer<T>, count: usize) {
        match count {
            0 => {}
            1 => {
                assert!(self.is_none());
                input.take(1, &mut |ptr| {
                    // SAFETY: Per the `TakeBuffer` implementation contract and
                    // the above assertion, the types match and we have been
                    // given ownership of the item.
                    unsafe { *self = Some(ptr.cast::<T>().read()) };
                });
            }
            _ => panic!(
                "cannot take more than {} item(s)",
                self.remaining_capacity()
            ),
        }
    }
}

/// A `WriteBuffer` implementation, backed by a `Vec`.
pub struct VecBuffer<T: Send + Sync + 'static> {
    buffer: Vec<MaybeUninit<T>>,
    offset: usize,
}

impl<T: Send + Sync + 'static> VecBuffer<T> {
    /// Create a new instance with the specified capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            buffer: Vec::with_capacity(capacity),
            offset: 0,
        }
    }

    /// Reset the state of this buffer, removing all items and preserving its
    /// capacity.
    pub fn reset(&mut self) {
        self.skip(self.remaining().len());
        self.buffer.clear();
        self.offset = 0;
    }
}

impl<T: Send + Sync + 'static> WriteBuffer<T> for VecBuffer<T> {
    fn remaining(&self) -> &[T] {
        // SAFETY: This relies on the invariant (upheld in the other methods of
        // this type) that all the elements from `self.offset` onward are
        // initialized and valid for `self.buffer`.
        unsafe { mem::transmute::<&[MaybeUninit<T>], &[T]>(&self.buffer[self.offset..]) }
    }

    fn skip(&mut self, count: usize) {
        assert!(count <= self.remaining().len());
        // SAFETY: See comment in `Self::remaining_`
        for item in &mut self.buffer[self.offset..][..count] {
            drop(unsafe { item.as_mut_ptr().read() })
        }
        self.offset = self.offset.checked_add(count).unwrap();
    }

    fn take(&mut self, count: usize, fun: &mut dyn FnMut(*const T)) {
        assert!(count <= self.remaining().len());
        fun(self.remaining().as_ptr());
        self.offset = self.offset.checked_add(count).unwrap();
    }
}

impl<T: Send + Sync + 'static> From<Vec<T>> for VecBuffer<T> {
    fn from(buffer: Vec<T>) -> Self {
        Self {
            // SAFETY: Transmuting from `Vec<T>` to `Vec<MaybeUninit<T>>` should
            // be sound for any `T`.
            buffer: unsafe { mem::transmute::<Vec<T>, Vec<MaybeUninit<T>>>(buffer) },
            offset: 0,
        }
    }
}

impl<T: Send + Sync + 'static> Drop for VecBuffer<T> {
    fn drop(&mut self) {
        self.reset();
    }
}

impl<T: Send + Sync + 'static> ReadBuffer<T> for Vec<T> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        Extend::extend(self, iter)
    }

    fn remaining_capacity(&self) -> usize {
        self.capacity().checked_sub(self.len()).unwrap()
    }

    fn move_from(&mut self, input: &mut dyn WriteBuffer<T>, count: usize) {
        assert!(count <= self.remaining_capacity());
        input.take(count, &mut |ptr| {
            // SAFETY: Per the `TakeBuffer` implementation contract and the
            // above assertion, the types match and we have been given ownership
            // of the items.
            unsafe {
                ptr::copy(ptr, self.as_mut_ptr().add(self.len()), count);
                self.set_len(self.len() + count);
            }
        });
    }
}

impl WriteBuffer<u8> for Cursor<Bytes> {
    fn remaining(&self) -> &[u8] {
        &self.get_ref()[usize::try_from(self.position()).unwrap()..]
    }

    fn skip(&mut self, count: usize) {
        assert!(
            count <= self.remaining().len(),
            "tried to skip {count} with {} remaining",
            self.remaining().len()
        );
        self.set_position(
            self.position()
                .checked_add(u64::try_from(count).unwrap())
                .unwrap(),
        );
    }

    fn take(&mut self, count: usize, fun: &mut dyn FnMut(*const u8)) {
        assert!(count <= self.remaining().len());
        fun(self.remaining().as_ptr());
        self.skip(count);
    }
}

impl WriteBuffer<u8> for Cursor<BytesMut> {
    fn remaining(&self) -> &[u8] {
        &self.get_ref()[usize::try_from(self.position()).unwrap()..]
    }

    fn skip(&mut self, count: usize) {
        assert!(count <= self.remaining().len());
        self.set_position(
            self.position()
                .checked_add(u64::try_from(count).unwrap())
                .unwrap(),
        );
    }

    fn take(&mut self, count: usize, fun: &mut dyn FnMut(*const u8)) {
        assert!(count <= self.remaining().len());
        fun(self.remaining().as_ptr());
        self.skip(count);
    }
}

impl ReadBuffer<u8> for BytesMut {
    fn extend<I: IntoIterator<Item = u8>>(&mut self, iter: I) {
        Extend::extend(self, iter)
    }

    fn remaining_capacity(&self) -> usize {
        self.capacity().checked_sub(self.len()).unwrap()
    }

    fn move_from(&mut self, input: &mut dyn WriteBuffer<u8>, count: usize) {
        assert!(count <= self.remaining_capacity());
        input.take(count, &mut |ptr| {
            // SAFETY: Per the `TakeBuffer` implementation contract and the
            // above assertion, the types match.
            unsafe {
                ptr::copy(ptr, self.as_mut_ptr().add(self.len()), count);
                self.set_len(self.len() + count);
            }
        });
    }
}
