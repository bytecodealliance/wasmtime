use crate::prelude::*;
use crate::runtime::vm::{GcHeap, GcStore, I31};
use anyhow::{Context, Result};
use core::fmt;
use core::marker;
use core::num::NonZeroU32;
use wasmtime_environ::{VMGcKind, VMSharedTypeIndex};

/// The common header for all objects allocated in a GC heap.
///
/// This header is shared across all collectors, although particular collectors
/// may always add their own trailing fields to this header for all of their own
/// GC objects.
///
/// This is a bit-packed structure that logically has the following fields:
///
/// ```ignore
/// struct VMGcHeader {
///     // Highest 2 bits.
///     kind: VMGcKind,
///
///     // 30 bits available for the `GcRuntime` to make use of however it sees fit.
///     reserved: u30,
///
///     // The `VMSharedTypeIndex` for this GC object, if it isn't an
///     // `externref` (or an `externref` re-wrapped as an `anyref`). `None` is
///     // represented with `VMSharedTypeIndex::default()`.
///     ty: Option<VMSharedTypeIndex>,
/// }
/// ```
#[repr(transparent)]
pub struct VMGcHeader(u64);

unsafe impl GcHeapObject for VMGcHeader {
    #[inline]
    fn is(_: &VMGcHeader) -> bool {
        true
    }
}

impl VMGcHeader {
    /// Create the header for an `externref`.
    pub fn externref() -> Self {
        let kind = VMGcKind::ExternRef as u32;
        let upper = u64::from(kind) << 32;
        let lower = u64::from(u32::MAX);
        Self(upper | lower)
    }

    /// Get the kind of GC object that this is.
    pub fn kind(&self) -> VMGcKind {
        let upper = u32::try_from(self.0 >> 32).unwrap();
        VMGcKind::from_u32(upper)
    }

    /// Get the reserved 30 bits in this header.
    ///
    /// These are bits are reserved for `GcRuntime` implementations to make use
    /// of however they see fit.
    pub fn reserved_u30(&self) -> u32 {
        let upper = u32::try_from(self.0 >> 32).unwrap();
        upper & VMGcKind::UNUSED_MASK
    }

    /// Set the 30-bit reserved value.
    ///
    /// # Panics
    ///
    /// Panics if the given `value` has any of the upper 2 bits set.
    pub fn set_reserved_u30(&mut self, value: u32) {
        assert_eq!(
            value & VMGcKind::MASK,
            0,
            "VMGcHeader::set_reserved_u30 with value using more than 30 bits"
        );
        self.0 |= u64::from(value) << 32;
    }

    /// Set the 30-bit reserved value.
    ///
    /// # Safety
    ///
    /// The given `value` must only use the lower 30 bits; its upper 2 bits must
    /// be unset.
    pub unsafe fn unchecked_set_reserved_u30(&mut self, value: u32) {
        self.0 |= u64::from(value) << 32;
    }

    /// Get this object's specific concrete type.
    pub fn ty(&self) -> Option<VMSharedTypeIndex> {
        let lower_mask = u64::from(u32::MAX);
        let lower = u32::try_from(self.0 & lower_mask).unwrap();
        if lower == u32::MAX {
            None
        } else {
            Some(VMSharedTypeIndex::new(lower))
        }
    }
}

#[cfg(test)]
mod vm_gc_header_tests {
    use super::*;
    use std::mem;

    #[test]
    fn size_align() {
        assert_eq!(mem::size_of::<VMGcHeader>(), 8);
        assert_eq!(mem::align_of::<VMGcHeader>(), 8);
    }
}

/// A raw, unrooted GC reference.
///
/// A `VMGcRef` is either:
///
/// * A reference to some kind of object on the GC heap, but we don't know
///   exactly which kind without further reflection. Furthermore, this is not
///   actually a pointer, but a compact index into a Wasm GC heap.
///
/// * An `i31ref`: it doesn't actually reference an object in the GC heap, but
///   is instead an inline, unboxed 31-bit integer.
///
/// ## `VMGcRef` and GC Barriers
///
/// Depending on the garbage collector in use, cloning, writing, and dropping a
/// `VMGcRef` may require invoking GC barriers (little snippets of code provided
/// by the collector to ensure it is correctly tracking all GC references).
///
/// Therefore, to encourage correct usage of GC barriers, this type does *NOT*
/// implement `Clone` or `Copy`. Use `GcStore::clone_gc_ref`,
/// `GcStore::write_gc_ref`, and `GcStore::drop_gc_ref` to clone, write, and
/// drop `VMGcRef`s respectively.
///
/// As an escape hatch, if you really need to copy a `VMGcRef` without invoking
/// GC barriers and you understand why that will not lead to GC bugs in this
/// particular case, you can use the `unchecked_copy` method.
#[derive(Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct VMGcRef(NonZeroU32);

impl<T> From<TypedGcRef<T>> for VMGcRef {
    #[inline]
    fn from(value: TypedGcRef<T>) -> Self {
        value.gc_ref
    }
}

impl fmt::LowerHex for VMGcRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl fmt::UpperHex for VMGcRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl fmt::Pointer for VMGcRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:#x}", self)
    }
}

impl VMGcRef {
    /// The only type of valid `VMGcRef` is currently `VMExternRef`.
    ///
    /// Assert on this anywhere you are making that assumption, so that we know
    /// all the places to update when it no longer holds true.
    pub const ONLY_EXTERN_REF_AND_I31: bool = true;

    /// If this bit is set on a GC reference, then the GC reference is actually an
    /// unboxed `i31`.
    ///
    /// Must be kept in sync with `wasmtime_cranelift::I31_REF_DISCRIMINANT`.
    pub const I31_REF_DISCRIMINANT: u32 = 1;

    /// Create a new `VMGcRef` from the given raw u32 value.
    ///
    /// Does not discriminate between indices into a GC heap and `i31ref`s.
    ///
    /// Returns `None` for zero values.
    ///
    /// The given index should point to a valid GC-managed object within this
    /// reference's associated heap. Failure to uphold this will be memory safe,
    /// but will lead to general failures such as panics or incorrect results.
    pub fn from_raw_u32(raw: u32) -> Option<Self> {
        Some(Self::from_raw_non_zero_u32(NonZeroU32::new(raw)?))
    }

    /// Create a new `VMGcRef` from the given index into a GC heap.
    ///
    /// The given index should point to a valid GC-managed object within this
    /// reference's associated heap. Failure to uphold this will be memory safe,
    /// but will lead to general failures such as panics or incorrect results.
    ///
    /// Returns `None` when the index is not 2-byte aligned and therefore
    /// conflicts with the `i31ref` discriminant.
    pub fn from_heap_index(index: NonZeroU32) -> Option<Self> {
        if (index.get() & Self::I31_REF_DISCRIMINANT) == 0 {
            Some(Self::from_raw_non_zero_u32(index))
        } else {
            None
        }
    }

    /// Create a new `VMGcRef` from the given raw value.
    ///
    /// Does not discriminate between indices into a GC heap and `i31ref`s.
    pub fn from_raw_non_zero_u32(raw: NonZeroU32) -> Self {
        VMGcRef(raw)
    }

    /// Construct a new `VMGcRef` from an unboxed 31-bit integer.
    #[inline]
    pub fn from_i31(val: I31) -> Self {
        let val = (val.get_u32() << 1) | Self::I31_REF_DISCRIMINANT;
        debug_assert_ne!(val, 0);
        let non_zero = unsafe { NonZeroU32::new_unchecked(val) };
        VMGcRef::from_raw_non_zero_u32(non_zero)
    }

    /// Create a new `VMGcRef` from a raw `r64` value from Cranelift.
    ///
    /// Returns an error if `raw` cannot be losslessly converted from a `u64`
    /// into a `u32`.
    ///
    /// Returns `Ok(None)` if `raw` is zero (aka a "null" `VMGcRef`).
    ///
    /// This method only exists because we can't currently use Cranelift's `r32`
    /// type on 64-bit platforms. We should instead have a `from_r32` method.
    pub fn from_r64(raw: u64) -> Result<Option<Self>> {
        let raw = u32::try_from(raw & (u32::MAX as u64))
            .err2anyhow()
            .with_context(|| format!("invalid r64: {raw:#x} cannot be converted into a u32"))?;
        Ok(Self::from_raw_u32(raw))
    }

    /// Copy this `VMGcRef` without running the GC's clone barriers.
    ///
    /// Prefer calling `clone(&mut GcStore)` instead! This is mostly an internal
    /// escape hatch for collector implementations.
    ///
    /// Failure to run GC barriers when they would otherwise be necessary can
    /// lead to leaks, panics, and wrong results. It cannot lead to memory
    /// unsafety, however.
    pub fn unchecked_copy(&self) -> Self {
        VMGcRef(self.0)
    }

    /// Get this GC reference as a u32 index into its GC heap.
    ///
    /// Returns `None` for `i31ref`s.
    pub fn as_heap_index(&self) -> Option<NonZeroU32> {
        if self.is_i31() {
            None
        } else {
            Some(self.0)
        }
    }

    /// Get this GC reference as a raw u32 value, regardless whether it is
    /// actually a reference to a GC object or is an `i31ref`.
    pub fn as_raw_u32(&self) -> u32 {
        self.0.get()
    }

    /// Get this GC reference as a raw `r64` value for passing to Cranelift.
    ///
    /// This method only exists because we can't currently use Cranelift's `r32`
    /// type on 64-bit platforms. We should instead be able to pass `VMGcRef`
    /// into compiled code directly.
    pub fn into_r64(self) -> u64 {
        u64::from(self.0.get())
    }

    /// Get this GC reference as a raw `r64` value for passing to Cranelift.
    ///
    /// This method only exists because we can't currently use Cranelift's `r32`
    /// type on 64-bit platforms. We should instead be able to pass `VMGcRef`
    /// into compiled code directly.
    pub fn as_r64(&self) -> u64 {
        u64::from(self.0.get())
    }

    /// Creates a typed GC reference from `self`, checking that `self` actually
    /// is a `T`.
    ///
    /// If this is not a GC reference to a `T`, then `Err(self)` is returned.
    pub fn into_typed<T>(self, gc_heap: &impl GcHeap) -> Result<TypedGcRef<T>, Self>
    where
        T: GcHeapObject,
    {
        if self.is_i31() {
            return Err(self);
        }
        if T::is(gc_heap.header(&self)) {
            Ok(TypedGcRef {
                gc_ref: self,
                _phantom: marker::PhantomData,
            })
        } else {
            Err(self)
        }
    }

    /// Creates a typed GC reference without actually checking that `self` is a
    /// `T`.
    ///
    /// `self` should point to a `T` object. Failure to uphold this invariant is
    /// memory safe, but will lead to general incorrectness such as panics or
    /// wrong results.
    pub fn into_typed_unchecked<T>(self) -> TypedGcRef<T>
    where
        T: GcHeapObject,
    {
        debug_assert!(!self.is_i31());
        TypedGcRef {
            gc_ref: self,
            _phantom: marker::PhantomData,
        }
    }

    /// Borrow `self` as a typed GC reference, checking that `self` actually is
    /// a `T`.
    pub fn as_typed<T>(&self, gc_heap: &impl GcHeap) -> Option<&TypedGcRef<T>>
    where
        T: GcHeapObject,
    {
        if self.is_i31() {
            return None;
        }
        if T::is(gc_heap.header(&self)) {
            let ptr = self as *const VMGcRef;
            let ret = unsafe { &*ptr.cast() };
            assert!(matches!(
                ret,
                TypedGcRef {
                    gc_ref: VMGcRef(_),
                    _phantom
                }
            ));
            Some(ret)
        } else {
            None
        }
    }

    /// Creates a typed GC reference without actually checking that `self` is a
    /// `T`.
    ///
    /// `self` should point to a `T` object. Failure to uphold this invariant is
    /// memory safe, but will lead to general incorrectness such as panics or
    /// wrong results.
    pub fn as_typed_unchecked<T>(&self) -> &TypedGcRef<T>
    where
        T: GcHeapObject,
    {
        debug_assert!(!self.is_i31());
        let ptr = self as *const VMGcRef;
        let ret = unsafe { &*ptr.cast() };
        assert!(matches!(
            ret,
            TypedGcRef {
                gc_ref: VMGcRef(_),
                _phantom
            }
        ));
        ret
    }

    /// Get a reference to the GC header that this GC reference is pointing to.
    ///
    /// Returns `None` when this is an `i31ref` and doesn't actually point to a
    /// GC header.
    pub fn gc_header<'a>(&self, gc_heap: &'a dyn GcHeap) -> Option<&'a VMGcHeader> {
        if self.is_i31() {
            None
        } else {
            Some(gc_heap.header(self))
        }
    }

    /// Is this `VMGcRef` actually an unboxed 31-bit integer, and not actually a
    /// GC reference?
    #[inline]
    pub fn is_i31(&self) -> bool {
        let val = self.0.get();
        (val & Self::I31_REF_DISCRIMINANT) != 0
    }

    /// Get the underlying `i31`, if any.
    #[inline]
    pub fn as_i31(&self) -> Option<I31> {
        if self.is_i31() {
            let val = self.0.get();
            Some(I31::wrapping_u32(val >> 1))
        } else {
            None
        }
    }

    /// Get the underlying `i31`, panicking if this is not an `i31`.
    #[inline]
    pub fn unwrap_i31(&self) -> I31 {
        self.as_i31().unwrap()
    }

    /// Is this `VMGcRef` a `VMExternRef`?
    #[inline]
    pub fn is_extern_ref(&self) -> bool {
        assert!(Self::ONLY_EXTERN_REF_AND_I31);
        !self.is_i31()
    }
}

/// A trait implemented by all objects allocated inside a GC heap.
///
/// # Safety
///
/// All implementations must:
///
/// * Be `repr(C)` or `repr(transparent)`
///
/// * Begin with a `VMGcHeader` as their first field
///
/// * Not have `Drop` implementations (aka, `std::mem::needs_drop::<Self>()`
///   should return `false`).
///
/// * Be memory safe to transmute to from an arbitrary byte sequence (that is,
///   it is okay if some bit patterns are invalid with regards to correctness,
///   so long as these invalid bit patterns cannot lead to memory unsafety).
pub unsafe trait GcHeapObject: Send + Sync {
    /// Check whether the GC object with the given header is an instance of
    /// `Self`.
    fn is(header: &VMGcHeader) -> bool;
}

/// A GC reference to a heap object of concrete type `T`.
///
/// Create typed GC refs via `VMGcRef::into_typed` and `VMGcRef::as_typed`, as
/// well as via their unchecked equivalents `VMGcRef::into_typed_unchecked` and
/// `VMGcRef::as_typed_unchecked`.
#[derive(Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct TypedGcRef<T> {
    gc_ref: VMGcRef,
    _phantom: marker::PhantomData<*mut T>,
}

impl<T> TypedGcRef<T>
where
    T: GcHeapObject,
{
    /// Clone this `VMGcRef`, running any GC barriers as necessary.
    pub fn clone(&self, gc_store: &mut GcStore) -> Self {
        Self {
            gc_ref: gc_store.clone_gc_ref(&self.gc_ref),
            _phantom: marker::PhantomData,
        }
    }

    /// Explicitly drop this GC reference, running any GC barriers as necessary.
    pub fn drop(self, gc_store: &mut GcStore) {
        gc_store.drop_gc_ref(self.gc_ref);
    }

    /// Copy this GC reference without running the GC's clone barriers.
    ///
    /// Prefer calling `clone(&mut GcStore)` instead! This is mostly an internal
    /// escape hatch for collector implementations.
    ///
    /// Failure to run GC barriers when they would otherwise be necessary can
    /// lead to leaks, panics, and wrong results. It cannot lead to memory
    /// unsafety, however.
    pub fn unchecked_copy(&self) -> Self {
        Self {
            gc_ref: self.gc_ref.unchecked_copy(),
            _phantom: marker::PhantomData,
        }
    }
}

impl<T> TypedGcRef<T> {
    /// Get the untyped version of this GC reference.
    pub fn as_untyped(&self) -> &VMGcRef {
        &self.gc_ref
    }
}
