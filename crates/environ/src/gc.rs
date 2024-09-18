//! Target- and pointer-width-agnostic definitions of GC-related types and
//! constants.
//!
//! These definitions are suitable for use both during compilation and at
//! runtime.
//!
//! Note: We don't bother gating these on `cfg(feature = "gc")` because that
//! makes downstream uses pretty annoying, and the primary thing we want to gate
//! on our various `gc` cargo features is the actual garbage collection
//! functions and their associated impact on binary size anyways.

#[cfg(feature = "gc")]
pub mod drc;

use crate::prelude::*;
use core::alloc::Layout;
use wasmtime_types::{WasmArrayType, WasmStorageType, WasmStructType, WasmValType};

/// Discriminant to check whether GC reference is an `i31ref` or not.
pub const I31_DISCRIMINANT: u64 = 1;

/// A mask that can be used to check for non-null and non-i31ref GC references
/// with a single bitwise-and operation.
pub const NON_NULL_NON_I31_MASK: u64 = !I31_DISCRIMINANT;

/// Get the byte size of the given Wasm type when it is stored inside the GC
/// heap.
pub fn byte_size_of_wasm_ty_in_gc_heap(ty: &WasmStorageType) -> u32 {
    match ty {
        WasmStorageType::I8 => 1,
        WasmStorageType::I16 => 2,
        WasmStorageType::Val(ty) => match ty {
            WasmValType::I32 | WasmValType::F32 | WasmValType::Ref(_) => 4,
            WasmValType::I64 | WasmValType::F64 => 8,
            WasmValType::V128 => 16,
        },
    }
}

/// A trait for getting the layout of a Wasm GC struct or array inside a
/// particular collector.
pub trait GcTypeLayouts {
    /// Get this collector's layout for the given array type.
    fn array_layout(&self, ty: &WasmArrayType) -> GcArrayLayout;

    /// Get this collector's layout for the given struct type.
    fn struct_layout(&self, ty: &WasmStructType) -> GcStructLayout;
}

/// The layout of a GC-managed object.
#[derive(Clone, Debug)]
pub enum GcLayout {
    /// The layout of a GC-managed array object.
    Array(GcArrayLayout),

    /// The layout of a GC-managed struct object.
    Struct(GcStructLayout),
}

impl From<GcArrayLayout> for GcLayout {
    fn from(layout: GcArrayLayout) -> Self {
        Self::Array(layout)
    }
}

impl From<GcStructLayout> for GcLayout {
    fn from(layout: GcStructLayout) -> Self {
        Self::Struct(layout)
    }
}

impl GcLayout {
    /// Get the underlying `GcStructLayout`, or panic.
    #[track_caller]
    pub fn unwrap_struct(&self) -> &GcStructLayout {
        match self {
            Self::Struct(s) => s,
            _ => panic!("GcLayout::unwrap_struct on non-struct GC layout"),
        }
    }

    /// Get the underlying `GcArrayLayout`, or panic.
    #[track_caller]
    pub fn unwrap_array(&self) -> &GcArrayLayout {
        match self {
            Self::Array(a) => a,
            _ => panic!("GcLayout::unwrap_array on non-array GC layout"),
        }
    }
}

/// The layout of a GC-managed array.
///
/// This layout is only valid for use with the GC runtime that created it. It is
/// not valid to use one GC runtime's layout with another GC runtime, doing so
/// is memory safe but will lead to general incorrectness like panics and wrong
/// results.
///
/// All offsets are from the start of the object; that is, the size of the GC
/// header (for example) is included in the offset.
///
/// All arrays are composed of the generic `VMGcHeader`, followed by
/// collector-specific fields, followed by the contiguous array elements
/// themselves. The array elements must be aligned to the element type's natural
/// alignment.
#[derive(Clone, Debug)]
#[allow(dead_code)] // Not used yet, but added for completeness.
pub struct GcArrayLayout {
    /// The size of this array object, ignoring its elements.
    pub size: u32,

    /// The alignment of this array.
    pub align: u32,

    /// The offset of the array's length.
    pub length_field_offset: u32,

    /// The offset from where this array's contiguous elements begin.
    pub elems_offset: u32,

    /// The size and natural alignment of each element in this array.
    pub elem_size: u32,
}

impl GcArrayLayout {
    /// Get the total size of this array for a given length of elements.
    pub fn size_for_len(&self, len: u32) -> u32 {
        self.size + len * self.elem_size
    }

    /// Get the offset of the `i`th element in an array with this layout.
    #[inline]
    pub fn elem_offset(&self, i: u32, elem_size: u32) -> u32 {
        self.elems_offset + i * elem_size
    }

    /// Get a `core::alloc::Layout` for an array of this type with the given
    /// length.
    pub fn layout(&self, len: u32) -> Layout {
        let size = self.size_for_len(len);
        let size = usize::try_from(size).unwrap();
        let align = usize::try_from(self.align).unwrap();
        Layout::from_size_align(size, align).unwrap()
    }
}

/// The layout for a GC-managed struct type.
///
/// This layout is only valid for use with the GC runtime that created it. It is
/// not valid to use one GC runtime's layout with another GC runtime, doing so
/// is memory safe but will lead to general incorrectness like panics and wrong
/// results.
///
/// All offsets are from the start of the object; that is, the size of the GC
/// header (for example) is included in the offset.
#[derive(Clone, Debug)]
pub struct GcStructLayout {
    /// The size (in bytes) of this struct.
    pub size: u32,

    /// The alignment (in bytes) of this struct.
    pub align: u32,

    /// The fields of this struct. The `i`th entry is the `i`th struct field's
    /// offset (in bytes) in the struct.
    pub fields: Vec<u32>,
}

impl GcStructLayout {
    /// Get a `core::alloc::Layout` for a struct of this type.
    pub fn layout(&self) -> Layout {
        let size = usize::try_from(self.size).unwrap();
        let align = usize::try_from(self.align).unwrap();
        Layout::from_size_align(size, align).unwrap()
    }
}

/// The kind of an object in a GC heap.
///
/// Note that this type is accessed from Wasm JIT code.
///
/// `VMGcKind` is a bitset where to test if `a` is a subtype of an
/// "abstract-ish" type `b`, we can simply use a single bitwise-and operation:
///
/// ```ignore
/// a <: b   iff   a & b == b
/// ```
///
/// For example, because `VMGcKind::AnyRef` has the high bit set, every kind
/// representing some subtype of `anyref` also has its high bit set.
///
/// We say "abstract-ish" type because in addition to the abstract heap types
/// (other than `i31`) we also have variants for `externref`s that have been
/// converted into an `anyref` via `extern.convert_any` and `externref`s that
/// have been converted into an `anyref` via `any.convert_extern`. Note that in
/// the latter case, because `any.convert_extern $foo` produces a value that is
/// not an instance of `eqref`, `VMGcKind::AnyOfExternRef & VMGcKind::EqRef !=
/// VMGcKind::EqRef`.
///
/// Furthermore, this type only uses the highest 6 bits of its `u32`
/// representation, allowing the lower 26 bytes to be bitpacked with other stuff
/// as users see fit.
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[rustfmt::skip]
#[allow(missing_docs)]
pub enum VMGcKind {
    ExternRef      = 0b010000 << 26,
    ExternOfAnyRef = 0b011000 << 26,
    AnyRef         = 0b100000 << 26,
    AnyOfExternRef = 0b100100 << 26,
    EqRef          = 0b101000 << 26,
    ArrayRef       = 0b101001 << 26,
    StructRef      = 0b101010 << 26,
}

impl VMGcKind {
    /// Mask this value with a `u32` to get just the bits that `VMGcKind` uses.
    pub const MASK: u32 = 0b111111 << 26;

    /// Mask this value with a `u32` that potentially contains a `VMGcKind` to
    /// get the bits that `VMGcKind` doesn't use.
    pub const UNUSED_MASK: u32 = !Self::MASK;

    /// Convert the given value into a `VMGcKind` by masking off the unused
    /// bottom bits.
    pub fn from_high_bits_of_u32(val: u32) -> VMGcKind {
        let masked = val & Self::MASK;
        match masked {
            x if x == Self::ExternRef.as_u32() => Self::ExternRef,
            x if x == Self::ExternOfAnyRef.as_u32() => Self::ExternOfAnyRef,
            x if x == Self::AnyRef.as_u32() => Self::AnyRef,
            x if x == Self::AnyOfExternRef.as_u32() => Self::AnyOfExternRef,
            x if x == Self::EqRef.as_u32() => Self::EqRef,
            x if x == Self::ArrayRef.as_u32() => Self::ArrayRef,
            x if x == Self::StructRef.as_u32() => Self::StructRef,
            _ => panic!("invalid `VMGcKind`: {masked:#032b}"),
        }
    }

    /// Does this kind match the other kind?
    ///
    /// That is, is this kind a subtype of the other kind?
    #[inline]
    pub fn matches(self, other: Self) -> bool {
        (self.as_u32() & other.as_u32()) == other.as_u32()
    }

    /// Get this `VMGcKind` as a raw `u32`.
    #[inline]
    pub fn as_u32(self) -> u32 {
        self as u32
    }
}

#[cfg(test)]
mod tests {
    use super::VMGcKind::*;
    use crate::prelude::*;

    #[test]
    fn kind_matches() {
        let all = [
            ExternRef,
            ExternOfAnyRef,
            AnyRef,
            AnyOfExternRef,
            EqRef,
            ArrayRef,
            StructRef,
        ];

        for (sup, subs) in [
            (ExternRef, vec![ExternOfAnyRef]),
            (ExternOfAnyRef, vec![]),
            (AnyRef, vec![AnyOfExternRef, EqRef, ArrayRef, StructRef]),
            (AnyOfExternRef, vec![]),
            (EqRef, vec![ArrayRef, StructRef]),
            (ArrayRef, vec![]),
            (StructRef, vec![]),
        ] {
            assert!(sup.matches(sup));
            for sub in &subs {
                assert!(sub.matches(sup));
            }
            for kind in all.iter().filter(|k| **k != sup && !subs.contains(k)) {
                assert!(!kind.matches(sup));
            }
        }
    }
}
